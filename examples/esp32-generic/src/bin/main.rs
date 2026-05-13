#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{
    clock::CpuClock,
    main,
};

use esp_hal::{
    delay::Delay,
    uart,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use serprog::Serprog;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32 -o esp32-wroom-32e -o vscode

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Create UART0
    let mut serial = uart::Uart::new(
        peripherals.UART0,
        uart::Config::default()
            .with_baudrate(115200),
    )
    .unwrap();

    // Create Serprog instance
    let delay = Delay::new();
    let mut serprog = Serprog::new(delay);
    let mut tx_buf = [0u8; serprog::SPI_BUFFER_SIZE as usize];

    // Configure SPI pins
    let sclk = peripherals.GPIO0;
    let miso_mosi = peripherals.GPIO2;
    let cs = peripherals.GPIO5;

    let miso = unsafe { miso_mosi.clone_unchecked() };

    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)        // order matters
    .with_mosi(miso_mosi)   // order matters
    .with_cs(cs);

    serial.write(b"Serprog ready\r\n").unwrap();

    loop {
        // Read one byte
        let mut rx_buf = [0u8; 1];
        serial.read(&mut rx_buf).unwrap();

        let byte = rx_buf[0];

        // Process command
        if let Some(response) = serprog.process_byte(byte, &mut spi, None) {
            let response_bytes = response.to_bytes(&mut tx_buf);

            // Write response
            serial.write(response_bytes).unwrap();

            // Wait until sent
            serial.flush().unwrap();
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
