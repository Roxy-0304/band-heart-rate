[English](README_EN.md) | [中文](README.md)

[![CI](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml/badge.svg)](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Roxy-0304/band-heart-rate)](https://github.com/Roxy-0304/band-heart-rate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ Disclaimer

> This project is forked from [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate), code written by AI.

## About

**Band Heart Rate Monitor** is a lightweight system tray heart rate monitoring application built with Rust. It receives real-time heart rate data from wearable devices via the standard BLE Heart Rate Service (UUID 0x180D). A built-in HTTP server supports REST and SSE real-time push, making it easy to integrate with live stream overlays (OBS).

You need to enable the heart rate broadcast function in your wearable device's settings.

> Latest builds can be downloaded from [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases).

## Features

- **System Tray Operation** — Starts silently to tray by default, no window, minimal resource usage
- **Windows Notifications** — Real-time notifications for scanning, connected, disconnected, timeout, and errors
- **Tray Menu** — Left-click opens Web UI, right-click menu: Open Web UI / Copy Address / Quit
- **Real-time Heart Rate Display** — Web interface with large digits and heart SVG animation
- **OBS Overlay** — `overlay` mode provides transparent background heart rate overlay
- **HTTP API** — REST endpoints + SSE real-time push + settings read/write
- **Auto Reconnect** — Automatic scan and reconnect with exponential backoff
- **Native Windows Support** — macOS / Linux require additional platform dependencies

## Quick Start

### Download

Go to [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) to download the executable and run it directly.

- **Default**: Starts silently to system tray, no window
- **`--console` flag**: Shows terminal window with log output

### Build from Source

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# Full version (system tray + BLE + web server)
cargo build --release

# Headless version (HTTP API only, no BLE or tray)
cargo build --release --no-default-features
```

**Requirements:** [Rust toolchain](https://www.rust-lang.org/tools/install) (rustup recommended)

> **macOS / Linux users:** The tray version primarily targets Windows. The headless mode (`cargo build --release --no-default-features`) compiles on all platforms. To build the tray version on macOS/Linux, you need to install platform-specific system dependencies.

## Usage

1. Enable **Heart Rate Broadcast** in your band/watch settings
2. Ensure Bluetooth is enabled on your device
3. Run the program — it automatically scans and connects to heart rate devices
4. Tray icon shows real-time heart rate, status changes via Windows notifications
5. Left-click tray icon to open Web UI, right-click for menu

## Tray Menu

| Menu Item | Function |
|-----------|----------|
| Open Web UI | Opens heart rate monitor page in browser |
| Copy Address | Copies Web UI address to clipboard |
| Quit | Exits the application |

## System Notifications

| Notification | Trigger |
|--------------|---------|
| Scanning | Started scanning for Bluetooth devices |
| Connected | Successfully connected to device |
| Disconnected | Device disconnected, reconnecting |
| Timeout | Scan timed out, no device found |
| Error | Bluetooth adapter error etc. |

## HTTP API

| Endpoint | Description |
|----------|-------------|
| `GET /` | Web UI page |
| `GET /heart-rate` | Current heart rate as JSON |
| `GET /heart-rate-stream` | SSE real-time heart rate stream |
| `GET /settings` | Get current configuration |
| `PUT /settings` | Update configuration (JSON body) |
| `GET /health` | Health check |

Default address: `http://127.0.0.1:3030` (configurable in settings; random port if occupied).

## Configuration

Settings are saved in the system config directory (Windows: `%APPDATA%/band-heart-rate/settings.json`) and can also be modified via HTTP API.

| Setting | Description | Default |
|---------|-------------|---------|
| `max_heart_rate` | Max heart rate (affects zone calculation) | `190` |
| `allowed_devices` | Comma-separated device name keywords | `band,amazfit,watch,mi` |
| `server_port` | HTTP server port | `3030` |
| `auto_start` | Start on system boot | `false` |
| `minimize_to_tray` | Minimize to tray on close | `true` |

## Compatible Devices

Compatible with any wearable device that supports the standard BLE Heart Rate Service (UUID 0x180D).

Supported devices include: Xiaomi Mi Band, Honor Band, Huawei Band/Watch, Amazfit, Apple Watch, and more. Enable Heart Rate Broadcast in your device settings to be detected.

## Screenshots

**Console Log**

![Console Log](doc/1.png)

**Control Panel**

![Control Panel](doc/2.png)

## License

[MIT](LICENSE)