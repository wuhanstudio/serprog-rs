use embedded_hal::delay::DelayNs;
use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;

use rtt_target::rprintln;

use serprog::cmd;
use serprog::SerprogResponse;

pub const SERIAL_BUF_SIZE: u16 = 256;
pub const SPI_BUFFER_SIZE: u32 = 256;

/// =========================
/// State machine
/// =========================
enum SerprogState {
    Idle,

    SpiHeader { idx: usize },
    SpiData { remaining: usize },

    Delay { idx: u8, value: u32 },

    WaitBustype,
    WaitSpiFreq { idx: u8 },
    WaitPin
}

/// =========================
/// Main state
/// =========================
pub struct Serprog<Delay> {
    state: SerprogState,

    spi_buffer: [u8; SPI_BUFFER_SIZE as usize],
    spi_buffer_pos: usize,

    delay: Delay,
    delay_value: u32,
}

impl <Delay: DelayNs> Serprog<Delay> {
    pub fn new(delay: Delay) -> Self {
        Self {
            state: SerprogState::Idle,

            spi_buffer: [0; SPI_BUFFER_SIZE as usize],
            spi_buffer_pos: 0,

            delay: delay,
            delay_value: 0,
        }
    }

    /// =========================
    /// Entry point
    /// =========================
    pub fn process_byte<SPI, CS>(
        &mut self,
        byte: u8,
        spi: &mut SPI,
        cs: &mut CS,
    ) -> Option<SerprogResponse<'_>>
    where
        SPI: SpiBus<u8>,
        CS: OutputPin,
    {
        // rprintln!("Received byte: 0x{:02X}", byte);
        match &mut self.state {
            SerprogState::Idle => self.handle_command(byte),

            SerprogState::SpiHeader { idx } => {
                // rprintln!("SPI Header byte {}: 0x{:02X}", idx, byte);

                self.spi_buffer[*idx] = byte;
                *idx += 1;

                if *idx == 6 {
                    let write_len = (self.spi_buffer[0] as usize)
                        | ((self.spi_buffer[1] as usize) << 8)
                        | ((self.spi_buffer[2] as usize) << 16);

                    if write_len > 0 {
                        // rprintln!("SPI Write length: {}", write_len);
                        self.state = SerprogState::SpiData {
                            remaining: write_len,
                        };
                    } else {
                        return self.execute_spi(spi, cs);
                    }
                }
                None
            }

            SerprogState::SpiData { remaining } => {
                rprintln!("SPI Data byte ({} remaining): 0x{:02X}", remaining, byte);
                self.spi_buffer[6 + self.spi_buffer_pos] = byte;
                self.spi_buffer_pos += 1;
                *remaining -= 1;

                if *remaining == 0 {
                    // rprintln!("SPI Data complete");
                    self.state = SerprogState::Idle;
                    return self.execute_spi(spi, cs);
                }
                None
            }

            SerprogState::Delay { idx, value } => {
                *value |= (byte as u32) << (8 * (*idx as u32));
                *idx += 1;

                if *idx == 4 {
                    self.delay_value = *value;
                    self.state = SerprogState::Idle;
                    Some(SerprogResponse::Ack)
                } else {
                    None
                }
            }

            SerprogState::WaitSpiFreq { idx } => {
                self.spi_buffer[*idx as usize] = byte;
                *idx += 1;

                if *idx == 4 {
                    let resp = SerprogResponse::SpiFreq([
                        self.spi_buffer[0],
                        self.spi_buffer[1],
                        self.spi_buffer[2],
                        self.spi_buffer[3],
                    ]);
                    self.state = SerprogState::Idle;
                    Some(resp)
                } else {
                    None
                }
            }

            SerprogState::WaitBustype
            | SerprogState::WaitPin
            => {
                // All are 1-byte consume + ACK behavior (C-compatible)
                self.state = SerprogState::Idle;
                Some(SerprogResponse::Ack)
            }
        }
    }

    /// =========================
    /// Command handler
    /// =========================
    fn handle_command(&mut self, cmd: u8) -> Option<SerprogResponse<'_>> {
        match cmd {
            // Bank 0
            cmd::S_CMD_NOP => Some(SerprogResponse::Ack),
            cmd::S_CMD_Q_IFACE => Some(SerprogResponse::InterfaceVersion),
            cmd::S_CMD_Q_CMDMAP => Some(SerprogResponse::CommandMap(0x3F, 0xC9, 0x3F)),
            cmd::S_CMD_Q_PGMNAME => Some(SerprogResponse::ProgrammerName("stm32-serprog")),
            cmd::S_CMD_Q_SERBUF => Some(SerprogResponse::SerialBufferSize(SERIAL_BUF_SIZE)),
            cmd::S_CMD_Q_BUSTYPE => Some(SerprogResponse::BusTypes(0x08)), 
            cmd::S_CMD_Q_CHIPSIZE => Some(SerprogResponse::Nak),
            cmd::S_CMD_Q_OPBUF => Some(SerprogResponse::Nak),

            // Bank 1
            cmd::S_CMD_O_INIT => Some(SerprogResponse::Ack),
            cmd::S_CMD_O_WRITEB => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_WRITEN => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_DELAY => {
                self.state = SerprogState::Delay { idx: 0, value: 0 };
                None
            }
            cmd::S_CMD_O_EXEC => {
                self.delay.delay_ms(self.delay_value);
                Some(SerprogResponse::Ack)
            }

            // Bank 2
            cmd::S_CMD_SYNCNOP => Some(SerprogResponse::SyncNOP),
            cmd::S_CMD_Q_RDNMAXLEN => Some(SerprogResponse::ReadNMaxLen(SPI_BUFFER_SIZE)),
            cmd::S_CMD_S_BUSTYPE => {
                self.state = SerprogState::WaitBustype;
                None
            }
            cmd::S_CMD_O_SPIOP => {
                self.spi_buffer_pos = 0;
                self.state = SerprogState::SpiHeader { idx: 0 };
                None
            }
            cmd::S_CMD_S_SPI_FREQ => {
                self.state = SerprogState::WaitSpiFreq { idx: 0 };
                None
            }
            cmd::S_CMD_S_PIN_STATE => {
                self.state = SerprogState::WaitPin;
                None
            }
            cmd::S_CMD_S_SPI_CS => Some(SerprogResponse::Nak),
            cmd::S_CMD_S_SPI_MODE => Some(SerprogResponse::Nak),

            // Others
            cmd::S_CMD_S_CS_MODE => Some(SerprogResponse::Nak),

            _ => Some(SerprogResponse::Nak),
        }
    }

    /// =========================
    /// SPI execution
    /// =========================
    fn execute_spi<SPI, CS>(
        &mut self,
        spi: &mut SPI,
        cs: &mut CS,
    ) -> Option<SerprogResponse<'_>>
    where
        SPI: SpiBus<u8>,
        CS: OutputPin,
    {
        let write_len = (self.spi_buffer[0] as u32)
            | ((self.spi_buffer[1] as u32) << 8)
            | ((self.spi_buffer[2] as u32) << 16);

        let read_len = (self.spi_buffer[3] as u32)
            | ((self.spi_buffer[4] as u32) << 8)
            | ((self.spi_buffer[5] as u32) << 16);

        if write_len > SPI_BUFFER_SIZE || read_len > SPI_BUFFER_SIZE {
            return Some(SerprogResponse::Nak);
        }

        let _ = cs.set_low();

        for i in 0..(write_len as usize) {
            rprintln!("SPI Write byte {}: 0x{:02X} ", i, self.spi_buffer[6 + i]);
            let mut b = [self.spi_buffer[6 + i]];
            if spi.transfer_in_place(&mut b).is_err() {
                let _ = cs.set_high();
                return Some(SerprogResponse::Nak);
            }
        }

        // rprintln!("SPI write complete: {} bytes", write_len);

        for i in 0..(read_len as usize) {
            let mut b = [0xFF];
            let _ = spi.transfer_in_place(&mut b);
            self.spi_buffer[i+1] = b[0];
            rprintln!("SPI Read byte {}: 0x{:02X} ", i, b[0]);
        }

        let _ = cs.set_high();

        Some(SerprogResponse::ReadData(&self.spi_buffer[1..1 + (read_len as usize)]))
    }
}
