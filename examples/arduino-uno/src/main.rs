#![no_std]
#![no_main]

use panic_halt as _;

use nb::block;
use arduino_hal::prelude::*;
use arduino_hal::spi;

use serprog::Serprog;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let (mut spi, _) = arduino_hal::Spi::new(
        dp.SPI,
        pins.d13.into_output(),
        pins.d11.into_output(),
        pins.d12.into_pull_up_input(),
        pins.d10.into_output(),
        spi::Settings::default(),
    );

    let delay = arduino_hal::Delay::new();
    let mut serprog = Serprog::new(delay);
    let mut tx_buf = [0u8; serprog::SPI_BUFFER_SIZE as usize];

    loop {
        // Process each byte as a potential command
        // Read a byte from the serial connection
        let byte = nb::block!(serial.read()).unwrap_infallible();
        if let Some(response) = serprog.process_byte(byte, &mut spi, None) {
            let response_bytes = response.to_bytes(&mut tx_buf);
            // Send byte-by-byte response
            for &b in response_bytes {
                block!(serial.write(b)).ok();
            }
        }
    }
}
