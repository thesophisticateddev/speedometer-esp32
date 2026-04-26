# OBD2 Digital Dashboard

An automotive instrument cluster built with [Slint](https://slint.dev) and Rust.
Runs as a desktop simulator today; targets the ESP32-S3 with a real OBD-II
bridge tomorrow. Designed by following the phased plan in [`CLAUDE.md`](./CLAUDE.md).

![dashboard placeholder](docs/screenshot.png)

## Features

- Circular speedometer (0–180 km/h) and tachometer (0–8000 RPM) with animated
  needle, tick marks, and configurable green/yellow/red zones.
- Linear gauges for fuel and coolant temperature with mode-aware color palette
  (cold blue → normal green → warm yellow → hot red).
- Turn-signal and high-beam indicators with pulsing animation.
- Trip computer (distance, average speed, elapsed time) with reset.
- Warning bar that flashes on overheat (>110 °C), low fuel (<10 %), or
  overspeed (>120 km/h).
- Day/Night theme switch.
- 1 Hz timestamped console log via `chrono`.
- Hardware-abstraction trait so the same UI runs against the simulator today
  and against an ESP32 + ELM327 stack later.

## Prerequisites

- Rust 1.75+ (`rustup default stable`)
- Linux: `libfontconfig-dev`, `libxkbcommon-dev`, `libwayland-dev` or
  `libxcb-shape0-dev libxcb-xfixes0-dev` for X11
- macOS / Windows: no extra packages

## Build & run

```bash
# Default: simulated drive cycle
cargo run --release

# ESP32 hardware stub (returns zeros until the driver lands)
cargo run --release -- --hardware

# Standalone gauge stress test (cycles the full range in 30 s)
cargo run --release --bin test_gauges

# Tests (simulator unit tests)
cargo test
```

## Keyboard controls

| Key       | Action                                |
|-----------|---------------------------------------|
| `↑` / `↓` | Increase / decrease commanded speed   |
| `←` / `→` | Toggle left / right turn signal       |
| `H`       | Toggle high-beam                      |
| `N`       | Toggle night mode                     |
| `R`       | Reset trip computer                   |
| `F`       | Refuel to 100 %                       |
| `Q` / Esc | Quit                                  |

The window must hold focus for keys to register. After typing a key, the
simulator switches from autonomous drive cycle to user-commanded speed; press
`Q` and relaunch to return to the autonomous cycle.

## Project layout

```
ui/
  dashboard.slint     top-level Window
  components.slint    Gauge, LinearGauge, Indicator, TripComputer, WarningBar
src/
  main.rs             desktop entry point
  obd2_simulator.rs   physics model (+ unit tests)
  hardware.rs         DataSource trait, SimulatedSource, Esp32Source stub
  bin/test_gauges.rs  gauge stress test binary
docs/
  ESP32_SETUP.md      hardware notes
.github/workflows/
  ci.yml              fmt + clippy + build + test on Linux/macOS/Windows
```

## ESP32 target

See [`docs/ESP32_SETUP.md`](docs/ESP32_SETUP.md) for hardware list, pin
assignments, and wiring. The desktop binary already abstracts data acquisition
behind the `DataSource` trait, so the embedded port only needs to implement
that trait against the ELM327 UART driver — the UI layer is unchanged.

## License

Apache-2.0
