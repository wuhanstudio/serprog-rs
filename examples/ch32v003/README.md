## CH32V003

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

```
cargo install cargo-binutils
cargo objcopy --release  -- -O binary ch32v003-serprog.bin
```
