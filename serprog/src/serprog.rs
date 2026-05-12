use embedded_hal::delay::DelayNs;
use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;

use rtt_target::rprintln;

/// =========================
/// Protocol constants
/// =========================
const S_ACK: u8 = 0x06;
const S_NAK: u8 = 0x15;

pub const SERIAL_BUF_SIZE: u16 = 256;
pub const SPI_BUFFER_SIZE: u32 = 256;

/// =========================
/// Response type
/// =========================
#[derive(Debug)]
pub enum SerprogResponse<'a> {
    Ack,
    Nak,

    InterfaceVersion,
    CommandMap(u8, u8, u8),
    ProgrammerName(&'static str),
    SerialBufferSize(u16),
    BusTypes(u8),
    WriteNMaxLen(u32),
    ReadNMaxLen(u32),

    SyncNOP,
    SpiFreq([u8; 4]),

    ReadData(&'a [u8]),
}

/// =========================
/// Response encoding
/// =========================
impl<'a> SerprogResponse<'a> {
    pub fn to_bytes(&self, buf: &'a mut [u8]) -> &'a [u8] {
        match self {
            SerprogResponse::Ack => {
                buf[0] = S_ACK;
                &buf[..1]
            }

            SerprogResponse::Nak => {
                buf[0] = S_NAK;
                &buf[..1]
            }

            SerprogResponse::InterfaceVersion => {
                buf[0] = S_ACK;
                buf[1] = 0x01;
                buf[2] = 0x00;
                &buf[..3]
            }

            SerprogResponse::CommandMap(bank_0, bank_1, bank_2) => {
                let mut map = [0u8; 32];
                map[0] = *bank_0;
                map[1] = *bank_1;
                map[2] = *bank_2;

                buf[0] = S_ACK;
                buf[1..33].copy_from_slice(&map);
                &buf[..33]
            }

            SerprogResponse::ProgrammerName(name) => {
                buf[0] = S_ACK;

                let b = name.as_bytes();
                let len = b.len().min(16);

                buf[1..1 + len].copy_from_slice(&b[..len]);
                buf[1 + len..17].fill(0);

                &buf[..17]
            }

            SerprogResponse::SerialBufferSize(v) => {
                buf[0] = S_ACK;
                buf[1] = (*v & 0xFF) as u8;
                buf[2] = (*v >> 8) as u8;
                &buf[..3]
            }

            SerprogResponse::BusTypes(v) => {
                buf[0] = S_ACK;
                buf[1] = *v;
                &buf[..2]
            }

            SerprogResponse::WriteNMaxLen(v) => {
                buf[0] = S_ACK;
                buf[1] = (v & 0xFF) as u8;
                buf[2] = ((v >> 8) & 0xFF) as u8;
                buf[3] = ((v >> 16) & 0xFF) as u8;
                &buf[..4]
            }

            SerprogResponse::ReadNMaxLen(v) => {
                buf[0] = S_ACK;
                buf[1] = (v & 0xFF) as u8;
                buf[2] = ((v >> 8) & 0xFF) as u8;
                buf[3] = ((v >> 16) & 0xFF) as u8;
                &buf[..4]
            }

            SerprogResponse::SyncNOP => {
                buf[0] = S_NAK;
                buf[1] = S_ACK;
                &buf[..2]
            }

            SerprogResponse::SpiFreq(v) => {
                buf[0] = S_ACK;
                buf[1..5].copy_from_slice(v);
                &buf[..5]
            }

            SerprogResponse::ReadData(data) => {
                buf[0] = S_ACK;
                buf[1..1 + data.len()].copy_from_slice(data);
                &buf[..1 + data.len()]
            }
        }
    }
}

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
            0x00 => Some(SerprogResponse::Ack),
            0x01 => Some(SerprogResponse::InterfaceVersion),
            0x02 => Some(SerprogResponse::CommandMap(0x3F, 0xC9, 0x3F)),
            0x03 => Some(SerprogResponse::ProgrammerName("stm32-serprog")),
            0x04 => Some(SerprogResponse::SerialBufferSize(SERIAL_BUF_SIZE)),
            0x05 => Some(SerprogResponse::BusTypes(0x08)),
            0x06 => Some(SerprogResponse::Nak),
            0x07 => Some(SerprogResponse::Nak),

            // Bank 1
            0x08 => Some(SerprogResponse::WriteNMaxLen(SPI_BUFFER_SIZE)),
            0x09 => Some(SerprogResponse::Nak),
            0x0A => Some(SerprogResponse::ReadNMaxLen(SPI_BUFFER_SIZE)),
            0x0B => Some(SerprogResponse::Ack),
            0x0C => Some(SerprogResponse::Nak),
            0x0D => Some(SerprogResponse::Nak),
            0x0E => {
                self.state = SerprogState::Delay { idx: 0, value: 0 };
                None
            }
            0x0F => {
                self.delay.delay_ms(self.delay_value);
                Some(SerprogResponse::Ack)
            }

            // Bank 2
            0x10 => Some(SerprogResponse::SyncNOP),
            0x11 => Some(SerprogResponse::ReadNMaxLen(SPI_BUFFER_SIZE)),
            0x12 => {
                self.state = SerprogState::WaitBustype;
                None
            }
            0x13 => {
                self.spi_buffer_pos = 0;
                self.state = SerprogState::SpiHeader { idx: 0 };
                None
            }
            0x14 => {
                self.state = SerprogState::WaitSpiFreq { idx: 0 };
                None
            }
            0x15 => {
                self.state = SerprogState::WaitPin;
                None
            }
            0x16 => Some(SerprogResponse::Nak),
            0x17 => Some(SerprogResponse::Nak),

            // Others
            0x18 => Some(SerprogResponse::Nak),

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
