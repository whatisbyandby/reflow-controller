# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an embedded Rust reflow oven controller that supports two platforms:
- **rp2040**: Bare-metal embedded target (RP2040 microcontroller)
- **std**: Desktop simulation platform for testing

The project uses Embassy async runtime for both platforms, enabling shared core logic with platform-specific implementations.

## Build and Run Commands

### Testing (std platform)
```bash
# Build and run tests
cargo test

# Run simulation (requires running from platform/std directory)
cd platform/std && cargo run
```

### RP2040 Platform
```bash
# Build for RP2040 (requires running from platform/rp2040 directory)
cd platform/rp2040 && cargo build

# Build and flash to hardware with probe-rs
cd platform/rp2040 && cargo run

# Build release version
cd platform/rp2040 && cargo build --release
```

## Architecture

### Platform Abstraction Layer

The codebase uses conditional compilation to support both embedded and desktop platforms:

- Core library ([src/lib.rs](src/lib.rs)) is `no_std` compatible with optional `std` feature
- Platform-specific modules selected via feature flags:
  - `cfg(feature = "rp2040")`: Hardware implementations (GPIO, I2C, USB)
  - `cfg(feature = "std")`: Mock/simulation implementations
- Each module has platform variants (e.g., `inputs_rp2040.rs` / `inputs_std.rs`)

### Communication Architecture

The system uses Embassy channels and watches for inter-task communication:

- `INPUT_EVENT_CHANNEL`: Commands from USB/serial interface → controller
- `OUTPUT_COMMAND_CHANNEL`: Controller → output devices (fan, buzzer, lights)
- `HEATER_POWER`: Controller → heater control
- `CURRENT_STATE`: Controller state broadcasts to USB interface
- `PROFILE_LIST_CHANNEL`: SD card profile list → USB interface
- `ACTIVE_PROFILE_CHANNEL`: Profile data → controller

All channels are `static` with `CriticalSectionRawMutex` for cross-task safety.

### Task Structure

The application spawns concurrent tasks (async functions marked with `#[embassy_executor::task]`):

1. **controller_task**: Main state machine ([reflow_controller.rs](src/reflow_controller.rs))
2. **heater_task**: PWM/thermal simulation ([heater_rp2040.rs](src/heater_rp2040.rs) / [heater_std.rs](src/heater_std.rs))
3. **run_temperature_sensor**: MCP9600 I2C sensor or mock ([temperature_sensor_mcp9600.rs](src/temperature_sensor_mcp9600.rs) / [temperature_sensor_mock.rs](src/temperature_sensor_mock.rs))
4. **interface_task**: Button/door inputs ([inputs_rp2040.rs](src/inputs_rp2040.rs) / [inputs_std.rs](src/inputs_std.rs))
5. **output_task**: LEDs, buzzer, fan control ([outputs_rp2040.rs](src/outputs_rp2040.rs) / [outputs_std.rs](src/outputs_std.rs))
6. **usb_task**: USB/serial command interface ([usb_interface_rp2040.rs](src/usb_interface_rp2040.rs) / [usb_interface_std.rs](src/usb_interface_std.rs))

### Control System

- PID controller ([pid.rs](src/pid.rs)) with integral windup protection
- Reflow profiles ([profile.rs](src/profile.rs)) define temperature curves with 6 steps: Preheat, Soak, Ramp, ReflowRamp, ReflowCool, Cooling
- State machine in `ReflowController`: Initializing → Idle → Running → (Finished|Error)

## Serial Commands

The USB/serial interface accepts these text commands:
- `START`: Begin reflow process
- `STOP`: Stop current process
- `RESET`: Reset from finished/error state
- `LIST_PROFILES`: Request available profiles from SD card
- `SET_PROFILE <filename>`: Load a specific profile
- `q` (RP2040 only): Reset to USB bootloader mode

State broadcasts are JSON-serialized `ReflowControllerState` structs.

## Key Dependencies

- `embassy-*`: Async runtime, HAL, USB
- `serde` + `serde-json-core`: no_std JSON serialization
- `heapless`: Fixed-capacity collections (no heap allocation)
- `defmt` (rp2040) / `log` (std): Platform-specific logging
- `probe-rs`: RP2040 flashing and debugging

## Important Constraints

- All strings use `heapless::String<N>` with fixed capacity
- Profile names: max 32 chars, filenames: max 64 chars
- Maximum 16 profiles in list
- Heater power: 0-100% (u8)
- System tick: 100ms baseline (`SYSTEM_TICK_MILLIS`)

## Development Notes

- The main library is in [src/lib.rs](src/lib.rs) with re-exports of platform-specific modules
- Platform applications are in `platform/rp2040/` and `platform/std/` directories
- SD card support is prepared but not yet fully implemented ([sd_profile_reader.rs](src/sd_profile_reader.rs))
- RP2040 uses `assign-resources` macro to partition peripherals across tasks
- Temperature sensor is MCP9600 on I2C bus (shared with other devices)