# Raspi Pico2 (RP2350)

```
┌─────────────────────┐
│   Raspi Pico 2      |
│                     │
│  GPIO18 (SCK)  ─────┼──────── CLK    ┌──────────────┐
│  GPIO16 (MISO) ─────┼──────── DO     │              │
│  GPIO19 (MOSI) ─────┼──────── DI     │   W25Q64     │
│  GPIO17 (CS)   ─────┼──────── CS     │  SPI Flash   │
│  3.3V          ─────┼──────── VCC    │              │
│  GND           ─────┼──────── GND    └──────────────┘
│                     │
└──────┬──────────────┘
       │
       │ USB Cable
       ▼
   Computer
```

## Install `picotool`:

Download `picotool` for your OS:

https://github.com/raspberrypi/pico-sdk-tools/releases/tag/v2.2.0-3

```
rustup target add thumbv8m.main-none-eabihf
```
