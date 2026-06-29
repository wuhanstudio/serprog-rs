#![no_std]
#![no_main]

// CH32V003 with W25Q64 SPI Flash connected
// │  PC5 (SCK)  ────────┼──────── CLK    ┌──────────────┐
// │  PC7 (MISO) ────────┼──────── DO     │              │
// │  PC6 (MOSI) ────────┼──────── DI     │   W25Q64     │
// │  PD2 (CS)   ────────┼──────── CS     │  SPI Flash   │
// │  3.3V       ────────┼──────── VCC    │              │
// │  GND        ────────┼──────── GND    └──────────────┘
//
// Serial uses USART1: PD6 (RX), PD5 (TX)

use ch32_hal as hal;
use panic_halt as _;

use hal::gpio::{Level, Output};
use hal::spi::{Config as SpiConfig, Spi};
use hal::time::Hertz;
use hal::usart::{Config as UartConfig, Uart};

use serprog::Serprog;
// use embedded_hal::spi::SpiBus;

const SERIAL_BUF_SIZE: u16 = 256;

#[qingke_rt::entry]
fn main() -> ! {
    let mut cfg = hal::Config::default();
    cfg.rcc = hal::rcc::Config::SYSCLK_FREQ_48MHZ_HSE;
    let p = hal::init(cfg);

    let mut uart_cfg = UartConfig::default();
    uart_cfg.baudrate = 115_200;
    let mut uart = Uart::new_blocking(p.USART1, p.PD6, p.PD5, uart_cfg).unwrap();

    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = Hertz::mhz(1);
    let mut spi = Spi::new_blocking::<0>(p.SPI1, p.PC5, p.PC6, p.PC7, spi_cfg);

    let mut cs = Output::new(p.PD2, Level::High, Default::default());

    // Test SPI communication by querying the JEDEC ID of a connected SPI flash chip (e.g., W25Q32)
    // uart.blocking_write(b"0000000000").unwrap();
    // let mut data:[u8; 4] = [0x9f, 0x00, 0x00, 0x00];
    // let _ = spi.blocking_transfer_in_place(&mut data);
    // for byte in &data {
    //     uart.blocking_write(&[*byte]).unwrap();
    // }
    // uart.blocking_write(b"0000000000").unwrap();

    let mut tx_buf = [0u8; SERIAL_BUF_SIZE as usize];
    let spi_buffer = [0u8; (SERIAL_BUF_SIZE - 7) as usize];
    let mut serprog = Serprog::new(spi_buffer, "ch32v003-serprog");

    loop {
        let mut byte = [0u8; 1];
        if uart.blocking_read(&mut byte).is_err() {
            continue;
        }

        if let Some(response) = serprog.process_byte(
            byte[0],
            &mut spi,
            Some(&mut cs),
            &mut |b: u8| {
                let _ = uart.blocking_write(&[b]);
            },
        ) {
            let response_bytes = response.to_bytes(&mut tx_buf);
            let _ = uart.blocking_write(response_bytes);
        }
    }
}
