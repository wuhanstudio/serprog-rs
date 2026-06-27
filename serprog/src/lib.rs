#![no_std]

use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;

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
pub enum SerprogResponse {
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
}

/// =========================
/// Response encoding
/// =========================
impl SerprogResponse {
    pub fn to_bytes<'a>(&self, buf: &'a mut [u8]) -> &'a [u8] {
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

    WaitBustype,
    WaitSpiFreq { idx: u8 },
    WaitPin
}

/// =========================
/// Main state
/// =========================
pub struct Serprog<const SPI_BUF_SIZE: usize> {
    programmer: &'static str,
    state: SerprogState,

    spi_buffer: [u8; SPI_BUF_SIZE],
    spi_buffer_pos: usize,
}

// SPI_BUF_SIZE should be SerialBufferSize - 7 to fit in the protocol limits
// The 7 in SPI means cmd(1) + txamt(3) + rxamt(3) => 7

impl <const SPI_BUF_SIZE: usize> Serprog<SPI_BUF_SIZE> {
    pub fn new(spi_buffer: [u8; SPI_BUF_SIZE], programmer: &'static str) -> Self {

        Self {
            programmer: programmer,
            state: SerprogState::Idle,

            spi_buffer: spi_buffer,
            spi_buffer_pos: 0,
        }
    }

    /// =========================
    /// Entry point
    /// =========================
    pub fn process_byte<SPI, W>(
        &mut self,
        byte: u8,
        spi: &mut SPI,
        cs: Option<&mut dyn OutputPin<Error = core::convert::Infallible>>,
        write_byte: &mut W,
    ) -> Option<SerprogResponse>
    where
        SPI: SpiBus<u8>,
        W: FnMut(u8),
    {
        // rprintln!("Received byte: 0x{:02X}", byte);
        match &mut self.state {
            SerprogState::Idle => self.handle_command(byte),

            SerprogState::SpiHeader { idx } => {
                self.spi_buffer[*idx] = byte;
                *idx += 1;

                if *idx == 6 {
                    let write_len = (self.spi_buffer[0] as u32)
                        | ((self.spi_buffer[1] as u32) << 8)
                        | ((self.spi_buffer[2] as u32) << 16);

                    if write_len > 0 {
                        self.state = SerprogState::SpiData {
                            remaining: write_len as u32,
                        };
                    } else {
                        return self.execute_spi(spi, cs, write_byte);
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
                    return self.execute_spi(spi, cs, write_byte);
                }
                None
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
    fn handle_command(&mut self, cmd: u8) -> Option<SerprogResponse> {
        match cmd {
            // Bank 0
            cmd::S_CMD_NOP => Some(SerprogResponse::Ack),
            cmd::S_CMD_Q_IFACE => Some(SerprogResponse::InterfaceVersion),
            cmd::S_CMD_Q_CMDMAP => Some(SerprogResponse::CommandMap(0x3F, 0x00, 0x3D)),
            cmd::S_CMD_Q_PGMNAME => Some(SerprogResponse::ProgrammerName(self.programmer)),
            cmd::S_CMD_Q_SERBUF => Some(SerprogResponse::SerialBufferSize( (self.spi_buffer.len() + 7 as usize) as u16)),
            cmd::S_CMD_Q_BUSTYPE => Some(SerprogResponse::BusTypes(0x08)),
            cmd::S_CMD_Q_CHIPSIZE => Some(SerprogResponse::Nak),
            cmd::S_CMD_Q_OPBUF => Some(SerprogResponse::Nak),

            // Bank 1
            cmd::S_CMD_Q_WRNMAXLEN => Some(SerprogResponse::Nak),
            cmd::S_CMD_R_BYTE => Some(SerprogResponse::Nak),
            cmd::S_CMD_R_NBYTES => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_INIT => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_WRITEB => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_WRITEN => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_DELAY => Some(SerprogResponse::Nak),
            cmd::S_CMD_O_EXEC => Some(SerprogResponse::Nak),

            // Bank 2
            cmd::S_CMD_SYNCNOP => Some(SerprogResponse::SyncNOP),
            cmd::S_CMD_Q_RDNMAXLEN => Some(SerprogResponse::Nak),
            cmd::S_CMD_S_BUSTYPE => {
                self.state = SerprogState::WaitBustype;
                None
            }
            cmd::S_CMD_O_SPIOP => {
                // rprintln!("SPI operation command received");
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
    fn execute_spi<SPI, W>(
        &mut self,
        spi: &mut SPI,
        mut cs: Option<&mut dyn OutputPin<Error = core::convert::Infallible>>,
        write_byte: &mut W,
    ) -> Option<SerprogResponse>
    where
        SPI: SpiBus<u8>,
        W: FnMut(u8),
    {
        let write_len: u32 = (self.spi_buffer[0] as u32)
            | ((self.spi_buffer[1] as u32) << 8)
            | ((self.spi_buffer[2] as u32) << 16);

        let read_len = (self.spi_buffer[3] as u32)
            | ((self.spi_buffer[4] as u32) << 8)
            | ((self.spi_buffer[5] as u32) << 16);

        if let Some(cs) = cs.as_mut() {
            let _ = cs.set_low();
        }

        // TX phase: write data from buffer, discard received bytes
        // Layout: [0..5] header, [6..6+write_len] TX data
        let tx_start = 6;
        if write_len > 0 {
            if spi.write(&self.spi_buffer[tx_start..tx_start + write_len as usize]).is_err() {
                if let Some(cs) = cs.as_mut() {
                    let _ = cs.set_high();
                }
                return Some(SerprogResponse::Nak);
            }
        }

        // Send ACK before streaming read data (mirrors C reference behaviour)
        write_byte(S_ACK);

        // RX phase: stream bytes directly to the caller without buffering.
        // Send 0xFF as dummy MOSI bytes while clocking in MISO.
        let mut tmp = [0xFFu8; 1];
        for _ in 0..read_len {
            if spi.transfer_in_place(&mut tmp).is_err() {
                tmp[0] = 0x00;
            }
            write_byte(tmp[0]);
            tmp[0] = 0xFF; // restore dummy byte for next iteration
        }

        if let Some(cs) = cs.as_mut() {
            let _ = cs.set_high();
        }

        None // Response (ACK + data) was sent directly via write_byte
    }
}