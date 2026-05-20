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
// use embedded_hal::spi::SpiBus;

const SERIAL_BUF_SIZE: u16 = 256;

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
    let (tx_pin, rx_pin) = (peripherals.GPIO1, peripherals.GPIO3);
    let mut serial = uart::Uart::new(
        peripherals.UART0,
        uart::Config::default()
            .with_baudrate(115200),
    )
    .unwrap()
    .with_tx(tx_pin)
    .with_rx(rx_pin);

    // Configure SPI pins
    let sclk = peripherals.GPIO18;
    let miso = peripherals.GPIO19;
    let mosi = peripherals.GPIO23;
    let cs   = peripherals.GPIO17;

    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)
    .with_mosi(mosi)  
    .with_cs(cs);

    // Test SPI communication by querying the JEDEC ID of a connected SPI flash chip (e.g., W25Q32)
    // serial.write(b"0000000000").unwrap();
    // serial.flush().unwrap();

    // let mut data:[u8; 4] = [0x9f, 0x00, 0x00, 0x00];
    // spi.transfer_in_place(&mut data).unwrap();
    // for byte in &data {
    //     serial.write(&[*byte]).unwrap();
    // }

    // serial.write(b"0000000000").unwrap();
    // serial.flush().unwrap();

    // Create Serprog instance
    let delay = Delay::new();

    let mut rx_buf = [0u8; SERIAL_BUF_SIZE as usize];
    let mut tx_buf = [0u8; SERIAL_BUF_SIZE as usize];

    // The 7 in SPI means cmd(1) + txamt(3) + rxamt(3) => 7
    let spi_buffer = [0u8; (SERIAL_BUF_SIZE - 7) as usize];

    let mut serprog = Serprog::new(delay, spi_buffer);
    
    loop {
        // Read incoming data
        match serial.read(&mut rx_buf) {
            Ok(count) if count > 0 => {
                // Process each byte as a potential command
                for i in 0..count {
                    let byte = rx_buf[i];
                    if let Some(response) = serprog.process_byte(byte, &mut spi, None) {
                        let response_bytes = response.to_bytes(&mut tx_buf);
                        let _ = serial.write(response_bytes);
                    }
                }
            }
            _ => {}
        }
    }
}
