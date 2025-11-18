# MEATER reader

A Rust workspace for connecting to and reading data from [MEATER](https://www.meater.com) smart thermometers.

## Workspace Structure

This project consists of three crates:

- **`meater-core`**: Core library with BLE scanning, connection management, and data parsing
- **`meater-cli`**: Cross-platform command-line interface for scanning and connecting to MEATER probes
- **`meater-display`**: Raspberry Pi display application with SH1106 OLED support (Linux-only)

## Quick Start

### Cross-Platform CLI

Build and use the CLI tool on any platform:

```bash
# Build the CLI
cargo build --release -p meater-cli

# Scan for MEATER devices
./target/release/meater-cli scan

# Connect to a device and display live temperature
./target/release/meater-cli connect <MAC_ADDRESS>
```

### Building

To build the cross-platform components (core library and CLI):
```bash
cargo build --release
```

This will build `meater-core` and `meater-cli` by default.

To build everything including the Raspberry Pi display app (Linux only):
```bash
cargo build --workspace --release
```

## Raspberry Pi Display Application

Install the following `meater.service` into `/etc/systemd/system/meater.service`


```
[Unit]
Description=MEATER service
After=bluetooth.target

[Service]
ExecStart=/usr/bin/meater

[Install]
WantedBy=multi-user.target
```

to start the application as a systemd service automatically. In addition, edit
`/etc/bluetooth/main.conf` and change

```
DiscoverableTimeout = 0
```

to avoid losing the device after three minutes.


## Acknowledgements

Temperature conversion taken from the reverse engineering efforts by [Nathan
Faber](https://github.com/nathanfaber/meaterble).


## License

[MIT](./LICENSE)
