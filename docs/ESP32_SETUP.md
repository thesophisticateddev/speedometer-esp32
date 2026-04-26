# ESP32 Hardware Setup

This document captures the wiring and configuration for porting the dashboard
to an **ESP32-S3-Box-3** (320×240 LCD) reading OBD-II data via an **ELM327
USB/Bluetooth adapter**.

> The Rust side is already abstracted behind the `DataSource` trait
> (see `src/hardware.rs`). The ESP32 port only needs to replace
> `Esp32Source` with a real implementation; the UI layer stays as-is.

## Hardware list

| Item                       | Purpose                                  |
|----------------------------|------------------------------------------|
| ESP32-S3-Box-3             | MCU + 320×240 LCD + speaker + buttons    |
| ELM327 ELM-USB or BT4.0    | OBD-II ↔ UART / BLE bridge               |
| TJA1051 CAN transceiver    | Direct CAN-bus alternative to ELM327     |
| 2× momentary push buttons  | Left / right turn signal inputs          |
| 1× SPST toggle switch      | High-beam input                          |
| 12 V → 5 V buck converter  | Vehicle power                            |
| OBD-II splitter cable      | Tap into the car connector               |

## Pin assignments

```
ESP32-S3 GPIO   Function                Direction   Notes
─────────────   ─────────────────────   ─────────   ─────────────────────
GPIO 18         Left turn signal        in (PU)     active-low button
GPIO 19         Right turn signal       in (PU)     active-low button
GPIO 21         High-beam input         in (PU)     active-low toggle
GPIO 17 (TX)    ELM327 UART TX          out         to ELM327 RX
GPIO 18 → 16    ELM327 UART RX          in          (move turn signal pin
                                                    if using ELM327 UART)
GPIO 4          CAN TX (TJA1051)        out         optional, native CAN
GPIO 5          CAN RX (TJA1051)        in          optional, native CAN
```

If using the ELM327 over **UART**, configure the port for **38400 baud, 8-N-1,
no flow control**. The ELM327 firmware boots at 38400 by default; some clones
default to 9600 — check with `AT I` after connecting.

## Wiring diagram (ASCII)

```
            +12V (vehicle)
                │
        ┌───────┴──────┐
        │ buck 12→5 V  │
        └───────┬──────┘
                │ 5V
   ┌────────────┴───────────────┐
   │       ESP32-S3-Box-3       │
   │                            │
   │  GPIO17 ─── TX ──┐         │       ┌──────────┐
   │  GPIO16 ─── RX ──┼── UART ─┤ ELM327│── OBD-II ──→ car
   │                  │         └──────────┘
   │  GPIO18 ── L turn ──┐                    +5V
   │  GPIO19 ── R turn ──┤   ┌─[ btn ]─ GND
   │  GPIO21 ── high-bm ─┘   │
   │                         │
   │  GPIO4  ── CAN TX ──┐   ┌──────────┐
   │  GPIO5  ── CAN RX ──┼── │ TJA1051  │── CAN-H/L ─→ car
   │                     │   └──────────┘
   │  GND  ──────────────┴── GND
   └─────────────────────────────────────┘
```

## OBD-II PIDs to query (ELM327)

| PID    | Quantity            | Mapping in dashboard   |
|--------|---------------------|------------------------|
| 010D   | Vehicle speed       | `speed` (km/h)         |
| 010C   | Engine RPM          | `rpm` (rpm = A*256+B / 4) |
| 012F   | Fuel level          | `fuel` (% = A * 100/255) |
| 0105   | Coolant temperature | `coolant_temp` (°C = A − 40) |

Issue at ~5 Hz; the dashboard tick is 100 ms but most ELM327 clones cap at
about 10 transactions/sec on a hot bus.

## Build steps (planned)

The desktop crate compiles for `xtensa-esp32s3-none-elf` with these changes
once the embedded driver lands:

1. Add `[target.xtensa-esp32s3-none-elf]` profile in `.cargo/config.toml`.
2. Replace `slint` desktop backend with `slint = { version = "1.16",
   default-features = false, features = ["renderer-software", "compat-1-2"] }`.
3. Implement `DataSource` against `esp_idf_hal::uart` (ELM327) or
   `embedded-can` (TJA1051).
4. `cargo build --release --target xtensa-esp32s3-none-elf`.

The `ui/*.slint` files require no changes — they already render at
800×480 and scale down cleanly to the 320×240 LCD via a `Window { width:
320px; height: 240px; }` override component.

## Memory tips

- Build with `opt-level = "z"` and `lto = true` (already set in `Cargo.toml`).
- Disable `chrono` in the embedded build (use `esp_idf_svc::systime` instead).
- The 28-tick gauge is cheap (small fixed array); keep it.
- Drop the `WarningBar` opacity animation if frame budget is tight — replace
  with on/off visibility toggled at 2 Hz from Rust.
