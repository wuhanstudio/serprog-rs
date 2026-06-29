#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::spi::MODE_0;
use embedded_hal::digital::OutputPin;

use rp235x_hal::gpio::{FunctionSpi, FunctionUart};
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::clocks::Clock;
use rp235x_hal::fugit::RateExtU32;

use serprog::Serprog;

const SERIAL_BUF_SIZE: u16 = 256;

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[rp235x_hal::entry]
fn main() -> ! {
    let mut pac = rp235x_hal::pac::Peripherals::take().unwrap();

    let mut watchdog = rp235x_hal::Watchdog::new(pac.WATCHDOG);

    let clocks = rp235x_hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let sio = rp235x_hal::Sio::new(pac.SIO);

    let pins = rp235x_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // UART0 on GP0 (TX) and GP1 (RX)
    let uart_pins = (
        pins.gpio0.into_function::<FunctionUart>(),
        pins.gpio1.into_function::<FunctionUart>(),
    );

    let uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115_200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // SPI0 on GP19 (MOSI), GP16 (MISO), GP18 (SCK), with CS on GP17
    let spi_mosi = pins.gpio19.into_function::<FunctionSpi>();
    let spi_miso = pins.gpio16.into_function::<FunctionSpi>();
    let spi_sck = pins.gpio18.into_function::<FunctionSpi>();
    let mut cs = pins.gpio17.into_push_pull_output();
    cs.set_high().ok();

    let mut spi = rp235x_hal::spi::Spi::<_, _, _, 8>::new(
        pac.SPI0,
        (spi_mosi, spi_miso, spi_sck),
    )
    .init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16.MHz(),
        MODE_0,
    );

    let mut rx_buf = [0u8; SERIAL_BUF_SIZE as usize];
    let mut tx_buf = [0u8; SERIAL_BUF_SIZE as usize];

    // The 7 in SPI means cmd(1) + txamt(3) + rxamt(3) => 7
    let spi_buffer = [0u8; (SERIAL_BUF_SIZE - 7) as usize];
    let mut serprog = Serprog::new(spi_buffer, "pico2-serprog");

    loop {
        match uart.read_raw(&mut rx_buf) {
            Ok(count) if count > 0 => {
                for i in 0..count {
                    let byte = rx_buf[i];
                    if let Some(response) =
                        serprog.process_byte(byte, &mut spi, Some(&mut cs), &mut |b: u8| {
                            uart.write_full_blocking(&[b]);
                        })
                    {
                        let response_bytes = response.to_bytes(&mut tx_buf);
                        uart.write_full_blocking(response_bytes);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [rp235x_hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"Serprog over UART"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];
