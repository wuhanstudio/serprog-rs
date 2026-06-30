# ESP32 Generic

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
cargo install espup --locked
espup install
```
