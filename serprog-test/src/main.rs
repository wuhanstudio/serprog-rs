use anyhow::{bail, Result};
use serialport::SerialPort;
// use std::io::{Read, Write};
use std::time::Duration;

/// Change this to your serial device
const PORT: &str = "COM7";
const BAUD: u32 = 115200;

/// Serprog commands
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

/// Serprog responses
const S_ACK: u8 = 0x06;
const S_NAK: u8 = 0x15;

fn open_port() -> Result<Box<dyn SerialPort>> {
    let port = serialport::new(PORT, BAUD)
        .timeout(Duration::from_millis(500))
        .open()?;

    Ok(port)
}

fn send_cmd(
    port: &mut dyn SerialPort,
    cmd: &[u8],
    rx_len: usize,
) -> Result<Vec<u8>> {
    port.write_all(cmd)?;
    port.flush()?;

    let mut buf = vec![0u8; rx_len];
    port.read_exact(&mut buf)?;

    Ok(buf)
}

#[test]
fn test_00_nop() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_NOP], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

#[test]
fn test_01_query_interface_version() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_Q_IFACE], 3)?;

    if resp[0] != S_ACK {
        bail!("Expected ACK, got {:02x}", resp[0]);
    }

    let version = u16::from_le_bytes([resp[1], resp[2]]);

    print!("Interface version: {} ", version);

    assert!(version >= 1);

    Ok(())
}

#[test]
fn test_02_query_command_map() -> Result<()> {
    let mut port = open_port()?;

    // ACK + 32-byte bitmap
    let resp = send_cmd(&mut *port, &[S_CMD_Q_CMDMAP], 33)?;

    assert_eq!(resp[0], S_ACK);

    let cmdmap = &resp[1..];

    // Check that Q_IFACE command is supported
    let byte = cmdmap[(S_CMD_Q_IFACE / 8) as usize];
    let bit = 1 << (S_CMD_Q_IFACE % 8);

    assert!(
        (byte & bit) != 0,
        "Q_IFACE not advertised in command map"
    );

    Ok(())
}

#[test]
fn test_03_query_programmer_name() -> Result<()> {
    let mut port = open_port()?;

    // ACK + 16-byte string
    let resp = send_cmd(&mut *port, &[S_CMD_Q_PGMNAME], 17)?;

    assert_eq!(resp[0], S_ACK);

    let name = String::from_utf8_lossy(&resp[1..])
        .trim_matches(char::from(0))
        .to_string();

    print!("Programmer name: {} ", name);

    assert!(!name.is_empty());

    Ok(())
}

#[test]
fn test_04_query_serial_buffer_size() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_Q_SERBUF], 3)?;

    assert_eq!(resp[0], S_ACK);

    let size = u16::from_le_bytes([resp[1], resp[2]]);

    print!("Serial buffer size: {} ", size);

    assert!(size > 0);

    Ok(())
}

#[test]
fn test_05_query_bus_types() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_Q_BUSTYPE], 2)?;

    assert_eq!(resp[0], S_ACK);

    let bus_types = resp[1];

    print!("Supported bus types bitmask: {:02x} ", bus_types);

    assert!(bus_types != 0);

    Ok(())
}

// #[test]
// fn test_06_query_chip_size() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_Q_CHIPSIZE], 1)?;

//     assert_eq!(resp[0], S_NAK);

//     Ok(())
// }

#[test]
fn test_07_query_max_read_length() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_Q_RDNMAXLEN], 3)?;

    assert_eq!(resp[0], S_ACK);

    let max_len = u16::from_le_bytes([resp[1], resp[2]]);

    print!("Maximum read length: {} ", max_len);

    assert!(max_len > 0);

    Ok(())
}

// Bank 1

#[test]
fn test_08_query_max_write_length() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_Q_WRNMAXLEN], 3)?;

    assert_eq!(resp[0], S_ACK);

    let max_len = u16::from_le_bytes([resp[1], resp[2]]);

    print!("Maximum write length: {} ", max_len);

    assert!(max_len > 0);

    Ok(())
}

// #[test]
// fn test_09_read_byte() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_R_BYTE, 0x00, 0x00, 0x00], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

// #[test]
// fn test_0a_read_n_bytes() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_R_NBYTES, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

#[test]
fn test_0b_init_op() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_O_INIT], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

// #[test]
// fn test_0c_write_byte() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_O_WRITEB, 0x00, 0x00, 0x00, 0x00], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

// #[test]
// fn test_0d_write_n_bytes() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_O_WRITEN, 0x00, 0x00, 0x00], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

#[test]
fn test_0e_delay() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_O_DELAY, 0x01, 0x00, 0x00, 0x00], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

#[test]
fn test_0f_exec() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_O_EXEC], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

// Bank 2

#[test]
fn test_10_syncnop() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_SYNCNOP], 2)?;

    assert_eq!(resp[0], S_NAK, "Expected initial NAK");
    assert_eq!(resp[1], S_ACK, "Expected ACK");

    Ok(())
}

// #[test]
// fn test_11_query_read_max_length() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_Q_RDNMAXLEN], 3)?;

//     assert_eq!(resp[0], S_ACK);

//     let max_len = u16::from_le_bytes([resp[1], resp[2]]);

//     print!("Maximum read length: {} ", max_len);

//     assert!(max_len > 0);

//     Ok(())
// }

#[test]
fn test_12_set_bus_type() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_S_BUSTYPE, 0x01], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

#[test]
fn test_13_spi_op() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_O_SPIOP, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

#[test]
fn test_14_set_spi_freq() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_S_SPI_FREQ, 0x01, 0x00, 0x00, 0x00], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

#[test]
fn test_15_set_pin_state() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_S_PIN_STATE, 0x01], 1)?;

    assert_eq!(resp[0], S_ACK, "Expected ACK");

    Ok(())
}

// #[test]
// fn test_16_set_spi_cs() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_S_SPI_CS, 0x01], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

// #[test]
// fn test_17_set_spi_mode() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_S_SPI_MODE, 0x01], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }

// #[test]
// fn test_18_set_cs_mode() -> Result<()> {
//     let mut port = open_port()?;

//     let resp = send_cmd(&mut *port, &[S_CMD_S_CS_MODE, 0x01], 1)?;

//     assert_eq!(resp[0], S_NAK, "Expected NAK");

//     Ok(())
// }
