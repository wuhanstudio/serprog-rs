#![no_std]

use embedded_hal::delay::DelayNs;
use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;
// use rtt_target::rprintln;

pub const SPI_BUFFER_SIZE: u32 = 250;

/// =========================
/// Protocol constants
/// =========================

pub const S_ACK: u8 = 0x06;
pub const S_NAK: u8 = 0x15;

pub mod cmd {
    pub const S_CMD_NOP: u8 = 0x00;
    pub const S_CMD_Q_IFACE: u8 = 0x01;
    pub const S_CMD_Q_CMDMAP: u8 = 0x02;
    pub const S_CMD_Q_PGMNAME: u8 = 0x03;
    pub const S_CMD_Q_SERBUF: u8 = 0x04;
    pub const S_CMD_Q_BUSTYPE: u8 = 0x05;
    pub const S_CMD_Q_CHIPSIZE: u8 = 0x06;
    pub const S_CMD_Q_OPBUF: u8 = 0x07;
    pub const S_CMD_Q_WRNMAXLEN: u8 = 0x08;
    pub const S_CMD_R_BYTE: u8 = 0x09;
    pub const S_CMD_R_NBYTES: u8 = 0x0A;
    pub const S_CMD_O_INIT: u8 = 0x0B;
    pub const S_CMD_O_WRITEB: u8 = 0x0C;
    pub const S_CMD_O_WRITEN: u8 = 0x0D;
    pub const S_CMD_O_DELAY: u8 = 0x0E;
    pub const S_CMD_O_EXEC: u8 = 0x0F;
    pub const S_CMD_SYNCNOP: u8 = 0x10;
    pub const S_CMD_Q_RDNMAXLEN: u8 = 0x11;
    pub const S_CMD_S_BUSTYPE: u8 = 0x12;
    pub const S_CMD_O_SPIOP: u8 = 0x13;
    pub const S_CMD_S_SPI_FREQ: u8 = 0x14;
    pub const S_CMD_S_PIN_STATE: u8 = 0x15;
    pub const S_CMD_S_SPI_CS: u8 = 0x16;
    pub const S_CMD_S_SPI_MODE: u8 = 0x17;
    pub const S_CMD_S_CS_MODE: u8 = 0x18;
}

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
    SpiData { remaining: u32 },

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

    serial_buffer_size: u16,

    spi_buffer: [u8; SPI_BUFFER_SIZE as usize],
    spi_buffer_pos: usize,

    delay: Delay,
    delay_value: u32,
}

impl <Delay: DelayNs> Serprog<Delay> {
    pub fn new(delay: Delay, serial_buffer_size: u16) -> Self {
        Self {
            state: SerprogState::Idle,

            serial_buffer_size: serial_buffer_size,

            spi_buffer: [0; SPI_BUFFER_SIZE as usize],
            spi_buffer_pos: 0,

            delay: delay,
            delay_value: 0,
        }
    }

    /// =========================
    /// Entry point
    /// =========================
    pub fn process_byte<SPI>(
        &mut self,
        byte: u8,
        spi: &mut SPI,
        cs: Option<&mut dyn OutputPin<Error = core::convert::Infallible>>
    ) -> Option<SerprogResponse<'_>>
    where
        SPI: SpiBus<u8>
    {
        // rprintln!("Received byte: 0x{:02X}", byte);
        match &mut self.state {
            SerprogState::Idle => self.handle_command(byte),

            SerprogState::SpiHeader { idx } => {
                // rprintln!("SPI Header byte {}: 0x{:02X}", idx, byte);

                self.spi_buffer[*idx] = byte;
                *idx += 1;

                if *idx == 6 {
                    let write_len = (self.spi_buffer[0] as u32)
                        | ((self.spi_buffer[1] as u32) << 8)
                        | ((self.spi_buffer[2] as u32) << 16);

                    if write_len > 0 {
                        // rprintln!("SPI Write length: {}", write_len);
                        self.state = SerprogState::SpiData {
                            remaining: write_len as u32,
                        };
                    } else {
                        return self.execute_spi(spi, cs);
                    }
                }
                None
            }

            SerprogState::SpiData { remaining } => {
                // rprintln!("SPI Data byte ({} remaining): 0x{:02X}", remaining, byte);
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
            cmd::S_CMD_Q_SERBUF => Some(SerprogResponse::SerialBufferSize(self.serial_buffer_size)),
            cmd::S_CMD_Q_BUSTYPE => Some(SerprogResponse::BusTypes(0x08)),
            cmd::S_CMD_Q_CHIPSIZE => Some(SerprogResponse::Nak),
            cmd::S_CMD_Q_OPBUF => Some(SerprogResponse::Nak),

            // Bank 1
            cmd::S_CMD_Q_WRNMAXLEN => Some(SerprogResponse::WriteNMaxLen(SPI_BUFFER_SIZE)),
            cmd::S_CMD_R_BYTE => Some(SerprogResponse::Nak),
            cmd::S_CMD_R_NBYTES => Some(SerprogResponse::Nak),
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
    fn execute_spi<SPI>(
        &mut self,
        spi: &mut SPI,
        mut cs: Option<&mut dyn OutputPin<Error = core::convert::Infallible>>,
    ) -> Option<SerprogResponse<'_>>
    where
        SPI: SpiBus<u8>,
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

        let total_len = write_len as usize + read_len as usize;

        if let Some(cs) = cs.as_mut() {
            let _ = cs.set_low();
        }

        // Build a single transfer buffer in-place
        // TX part: actual data
        // RX part: dummy bytes (0xFF)
        let buf = &mut self.spi_buffer;

        // Shift layout:
        // [0..5] header
        // [6..6+write_len] TX
        // [6+write_len..6+write_len+read_len] dummy

        let tx_start = 6;
        let rx_start = 6 + write_len as usize;

        // ensure dummy bytes for read phase
        for i in 0..read_len as usize {
            buf[rx_start + i] = 0xFF;
        }

        let slice = &mut buf[tx_start..tx_start + total_len];

        if spi.transfer_in_place(slice).is_err() {
            if let Some(cs) = cs.as_mut() {
                let _ = cs.set_high();
            }
            return Some(SerprogResponse::Nak);
        }

        // RX data is now:
        let read_out = &buf[rx_start..rx_start + read_len as usize];

        if let Some(cs) = cs.as_mut() {
            let _ = cs.set_high();
        }

        Some(SerprogResponse::ReadData(read_out))
    }
}