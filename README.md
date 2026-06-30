# Serprog - Rust Implementation

A cross-platform Rust implementation of the flashrom serprog protocol that supports STM32, ESP32, Arduino UNO, Raspi Pico and CH32V. This allows you to use a MCU as an SPI flash programmer.

## Basic Setup - Reading W25Q64 Flash Chip

```
// STM32 Bluepill board (STM32F103C8T6)
┌─────────────────────┐
│   STM32 Blue Pill   │
│                     │
│  PA5 (SCK)  ────────┼──────── CLK    ┌──────────────┐
│  PA6 (MISO) ────────┼──────── DO     │              │
│  PA7 (MOSI) ────────┼──────── DI     │   W25Q64     │
│  PA4 (CS)   ────────┼──────── CS     │  SPI Flash   │
│  3.3V       ────────┼──────── VCC    │              │
│  GND        ────────┼──────── GND    └──────────────┘
│                     │
│  PA11 (USB D-)      │
│  PA12 (USB D+)      │
└──────┬──────────────┘
       │
       │ USB Cable
       ▼
   Computer
```

```
// Arduino UNO board
┌─────────────────────┐
│   Arduino UNO       |
│                     │
│  Pin 13 (SCK)  ────────┼──────── CLK    ┌──────────────┐
│  Pin 12 (MISO) ────────┼──────── DO     │              │
│  Pin 11 (MOSI) ────────┼──────── DI     │   W25Q64     │
│  Pin 10 (CS)   ────────┼──────── CS     │  SPI Flash   │
│  3.3V          ────────┼──────── VCC    │              │
│  GND           ────────┼──────── GND    └──────────────┘
│                     │
└──────┬──────────────┘
       │
       │ USB Cable
       ▼
   Computer
```

```
// Raspi Pico 2 Board (RP2350)
┌─────────────────────┐
│   Raspi Pico 2      |
│                     │
│  GPIO18 (SCK)  ────────┼──────── CLK    ┌──────────────┐
│  GPIO16 (MISO) ────────┼──────── DO     │              │
│  GPIO19 (MOSI) ────────┼──────── DI     │   W25Q64     │
│  GPIO17 (CS)   ────────┼──────── CS     │  SPI Flash   │
│  3.3V          ────────┼──────── VCC    │              │
│  GND           ────────┼──────── GND    └──────────────┘
│                     │
└──────┬──────────────┘
       │
       │ USB Cable
       ▼
   Computer
```

```
┌─────────────────────┐
│    ESP32 Generic    │
│                     │
│  GPIO18 (SCK)  ────────┼──────── CLK    ┌──────────────┐
│  GPIO19 (MISO) ────────┼──────── DO     │              │
│  GPIO23 (MOSI) ────────┼──────── DI     │   W25Q64     │
│  GPIO17 (CS)   ────────┼──────── CS     │  SPI Flash   │
│  3.3V          ────────┼──────── VCC    │              │
│  GND           ────────┼──────── GND    └──────────────┘
│                     │
└──────┬──────────────┘
       │
       │ USB Cable
       ▼
   Computer
```

```
// CH32V003 with W25Q64 SPI Flash connected
// │  PC5 (SCK)  ────────┼──────── CLK    ┌──────────────┐
// │  PC7 (MISO) ────────┼──────── DO     │              │
// │  PC6 (MOSI) ────────┼──────── DI     │   W25Q64     │
// │  PD2 (CS)   ────────┼──────── CS     │  SPI Flash   │
// │  3.3V       ────────┼──────── VCC    │              │
// │  GND        ────────┼──────── GND    └──────────────┘
//
// Serial uses USART1: PD6 (RX), PD5 (TX)
```

## Hardware Requirements

- STM32F103C8T6 Blue Pill board (or other supported boards)
- SPI flash chip (or device with SPI flash)
- USB cable (Mini or Micro USB depending on your Blue Pill)


## Building

### Prerequisites

1. Install Rust and cargo:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install probe-rs (for flashing):
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
```

### Build the firmware

Choose one of the MCUs in the [examples](examples/) folder:

```
examples:
├───arduino-uno
├───ch32v003
├───esp32-generic
├───raspi-pico2
└───stm32-bluepill
```

```bash
cd stm32-bluepill
cargo build --release
```

The binary will be at `target/xxxxx/release/xxxxx-serprog`

## Flashing to the MCU

```bash
cargo run --release
```

## Usage with flashrom

```bash
sudo apt-get install flashrom
```

### 1. Connect the MCU to your computer via USB

### 2. Find the serial device
```bash
# Linux
ls /dev/ttyACM*
# Usually /dev/ttyACM0

# macOS
ls /dev/cu.usbmodem*

# Windows
# Check Device Manager for COM port (e.g., COM3)
```

### 3. Read flash chip ID
```bash
flashrom -p serprog:dev=/dev/ttyACM0:4000000
```

Expected output:
```
Found chip "Winbond W25Q64.V" (8192 kB, SPI) on serprog.
```

### 4. Read flash contents
```bash
flashrom -p serprog:dev=/dev/ttyACM0:115200 -r flash_backup.bin
```

### 5. Write to flash
```bash
flashrom -p serprog:dev=/dev/ttyACM0:115200 -w firmware.bin
```

### 6. Erase flash
```bash
flashrom -p serprog:dev=/dev/ttyACM0:115200 -E
```

### 7. Verify flash
```bash
flashrom -p serprog:dev=/dev/ttyACM0:115200 -v firmware.bin
```

## Common Flash Chips Supported

- Winbond W25Q series (W25Q16, W25Q32, W25Q64, W25Q128)
- Macronix MX25L series
- Micron/Numonyx M25P series
- Spansion S25FL series
- ISSI IS25LP/WP series
- GigaDevice GD25Q series
- And many more...

## References

- [flashrom serprog protocol](https://www.flashrom.org/Serprog)
