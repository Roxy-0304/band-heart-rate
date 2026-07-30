[English](README_EN.md) | [中文](README.md)

[![CI](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml/badge.svg)](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Roxy-0304/band-heart-rate)](https://github.com/Roxy-0304/band-heart-rate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ Disclaimer

> This project is forked from [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate), code written by AI.

## About

**Band Heart Rate Monitor** is a native desktop heart rate monitoring application built with Rust and Slint. It receives real-time heart rate data from wearable devices via the standard BLE Heart Rate Service (UUID 0x180D). A built-in HTTP server supports REST and SSE real-time push, making it easy to integrate with live stream overlays.

You need to enable the heart rate broadcast function in your wearable device's settings.

> 💡 Latest builds can be downloaded from [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases).

## Features

- **Slint Native Window** — Minimal CPU and memory usage, high frame rate
- **Real-time Heart Rate Display** — Large digits with real-time refresh, auto-detect Warmup/Fat Burn/Aerobic/Limit zones
- **Live Statistics** — Min/Max/Average heart rate with one-click reset
- **Settings Panel** — Configure max heart rate, device keywords, HTTP port, auto-start, and minimize-to-tray
- **HTTP API** — REST endpoints + SSE real-time push + settings read/write
- **Auto Reconnect** — Automatic scan and reconnect with exponential backoff
- **Native Windows support** — macOS / Linux require additional platform dependencies

## Quick Start

### Download

Go to [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) to download the executable for your platform and run it directly.

### Build from Source

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# With GUI
cargo build --release

# Headless (HTTP API only)
cargo build --release --no-default-features
```

**Requirements:** [Rust toolchain](https://www.rust-lang.org/tools/install) (rustup recommended)

> 💡 **macOS / Linux users:** The GUI version primarily targets Windows. The headless mode (`cargo build --release --no-default-features`) compiles on all platforms. To build the GUI version on macOS/Linux, you need to install platform-specific system dependencies (e.g., `libdbus-1-dev`, `pkg-config`, `libwebkit2gtk-4.1-dev`, etc.).

## Usage

1. Enable **Heart Rate Broadcast** in your band/watch settings
2. Ensure Bluetooth is enabled on your device
3. Run the program — it automatically scans and connects to heart rate devices
4. Heart rate data is displayed in real-time in the native window and accessible via HTTP API
5. Click the gear icon in the top-right corner to open the settings panel

## HTTP API

| Endpoint | Description |
|----------|-------------|
| `GET /heart-rate` | Current heart rate as JSON |
| `GET /heart-rate-stream` | SSE real-time heart rate stream |
| `GET /settings` | Get current configuration |
| `PUT /settings` | Update configuration (JSON body) |
| `GET /health` | Health check |

Default address: `http://127.0.0.1:3030` (configurable in settings panel; random port if occupied).

## Configuration

Settings are saved in the system config directory (Windows: `%APPDATA%/band-heart-rate/settings.json`) and can also be modified via the settings panel or HTTP API.

| Setting | Description | Default |
|---------|-------------|---------|
| `max_heart_rate` | Max heart rate (affects zone calculation) | `190` |
| `allowed_devices` | Comma-separated device name keywords | `band,amazfit,watch,mi` |
| `server_port` | HTTP server port | `3030` |
| `auto_start` | Start on system boot | `false` |
| `minimize_to_tray` | Minimize to tray on close | `true` |

## Compatible Devices

Compatible with any wearable device that supports the standard BLE Heart Rate Service (UUID 0x180D). The GUI version is primarily tested on Windows 10/11; the headless mode supports all platforms.

Supported devices include: Xiaomi Mi Band, Honor Band, Huawei Band/Watch, Amazfit, Apple Watch, and more. Enable "Heart Rate Broadcast" in your device settings to be detected.

## Screenshots

**Main Interface**

![Main Interface](doc/1.png)

**Web Interface**

![Web Interface](doc/2.png)

**Settings Panel**

![Settings Panel](doc/3.png)

## License

[MIT](LICENSE)
