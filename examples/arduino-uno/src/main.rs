#![no_std]
#![no_main]

use panic_halt as _;

use nb::block;
use arduino_hal::prelude::*;

use serprog::Serprog;

const SERIAL_BUF_SIZE: u16 = 256;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let (mut spi, mut cs) = arduino_hal::Spi::new(
        dp.SPI,
        pins.d13.into_output(),
        pins.d11.into_output(),
        pins.d12.into_pull_up_input(),
        pins.d10.into_output(),
        arduino_hal::spi::Settings {
            data_order: arduino_hal::spi::DataOrder::MostSignificantFirst,
            clock: arduino_hal::spi::SerialClockRate::OscfOver16,
            mode: embedded_hal::spi::MODE_0,
        },
    );

    let delay = arduino_hal::Delay::new();
    let mut tx_buf = [0u8; SERIAL_BUF_SIZE as usize];

    // The 7 in SPI means cmd(1) + txamt(3) + rxamt(3) => 7
    let spi_buffer = [0u8; (SERIAL_BUF_SIZE - 7) as usize];

    let mut serprog = Serprog::new(delay, spi_buffer);

    loop {
        // Process each byte as a potential command
        // Read a byte from the serial connection
        let byte = nb::block!(serial.read()).unwrap_infallible();
        if let Some(response) = serprog.process_byte(byte, &mut spi, Some(&mut cs)) {
            let response_bytes = response.to_bytes(&mut tx_buf);
            // Send byte-by-byte response
            for &b in response_bytes {
                block!(serial.write(b)).ok();
            }
        }
    }
}
