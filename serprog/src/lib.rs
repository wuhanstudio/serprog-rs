#![no_std]

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

#[cfg(test)]
mod tests {
}
