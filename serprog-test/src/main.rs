use anyhow::{bail, Result};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;

/// Serprog commands
const S_CMD_NOP: u8 = 0x00;
const S_CMD_Q_IFACE: u8 = 0x01;
const S_CMD_Q_CMDMAP: u8 = 0x02;
const S_CMD_Q_PGMNAME: u8 = 0x03;
const S_CMD_SYNCNOP: u8 = 0x10;

/// Serprog responses
const S_ACK: u8 = 0x06;
const S_NAK: u8 = 0x15;

/// Change this to your serial device
const PORT: &str = "COM7";
const BAUD: u32 = 115200;

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

    println!("Interface version: {}", version);

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

    println!("Programmer name: {}", name);

    assert!(!name.is_empty());

    Ok(())
}

#[test]
fn test_10_syncnop() -> Result<()> {
    let mut port = open_port()?;

    let resp = send_cmd(&mut *port, &[S_CMD_SYNCNOP], 2)?;

    assert_eq!(resp[0], S_NAK, "Expected initial NAK");
    assert_eq!(resp[1], S_ACK, "Expected ACK");

    Ok(())
}
