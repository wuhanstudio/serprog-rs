#![no_std]
#![no_main]

// CH32V003 with W25Q64 SPI Flash connected
// │  PC5 (SCK)  ────────┼──────── CLK    ┌──────────────┐
// │  PC7 (MISO) ────────┼──────── DO     │              │
// │  PC6 (MOSI) ────────┼──────── DI     │   W25Q64     │
// │  PD2 (CS)   ────────┼──────── CS     │  SPI Flash   │
// │  3.3V       ────────┼──────── VCC    │              │
// │  GND        ────────┼──────── GND    └──────────────┘

// |  PD6 (RX)
// |  PD5 (TX)

use panic_halt as _;
use ch32_hal as hal;

use hal::delay::Delay;
use hal::gpio::{Level, Output};
use hal::usart::UartTx;

#[qingke_rt::entry]
fn main() -> ! {
    let mut config = hal::Config::default();
    config.rcc = hal::rcc::Config::SYSCLK_FREQ_48MHZ_HSE;
    let p = hal::init(config);

    let mut delay = Delay;
    let mut uart = UartTx::new_blocking(p.USART1, p.PD5, Default::default()).unwrap();
    let mut led = Output::new(p.PD6, Level::Low, Default::default());
    loop {
        led.toggle();
        let _ = uart.blocking_write(b"Hello, world!\r\n").unwrap();
        delay.delay_ms(1000);
    }
}
