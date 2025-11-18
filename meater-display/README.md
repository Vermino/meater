# MEATER Display

Raspberry Pi display application for MEATER smart thermometer.

## Platform Support

**This crate is Linux-only** and designed specifically for Raspberry Pi with an SH1106 OLED display.

### Building on Windows/macOS

This crate is not included in the default workspace build. When you run `cargo build` from the workspace root, only `meater-core` and `meater-cli` will be built.

If you try to build this crate explicitly on non-Linux platforms:
```bash
cargo build -p meater-display
```

It will compile but immediately exit with an error message directing you to use `meater-cli` instead.

###Building on Linux/Raspberry Pi

To build the display application on Linux:
```bash
cargo build -p meater-display --release
```

Or to build all workspace members including the display app:
```bash
cargo build --workspace --release
```

## Hardware Requirements

- Raspberry Pi (tested on Pi Zero)
- SH1106 128x64 OLED display
- I2C connection

## Installation

See the main [README](../README.md) for systemd service setup instructions.
