# Meater Tauri Application

Desktop application for Meater probe monitoring built with Tauri.

## Building

### Prerequisites
- Rust
- Node.js (for Tauri CLI)
- System dependencies for Tauri

### Install Tauri CLI
```bash
cargo install tauri-cli --version "^2.0.0"
```

### Development
```bash
# Start the web server in one terminal
cargo run --bin meater-web

# In another terminal, run Tauri dev
cd web/src-tauri
cargo tauri dev
```

### Build
```bash
cd web/src-tauri
cargo tauri build
```

## Features

- Native desktop application
- System tray integration
- Auto-start on boot (optional)
- Native notifications
- Better performance than browser
- Works offline with cached data

## Platform Support

- Windows
- macOS
- Linux

## Icons

Place your app icons in `web/src-tauri/icons/`:
- `32x32.png`
- `128x128.png`
- `128x128@2x.png`
- `icon.icns` (macOS)
- `icon.ico` (Windows)

You can use the flame emoji or create custom icons.
