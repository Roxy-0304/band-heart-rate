[English](README_EN.md) | [中文](README.md)

[![CI](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml/badge.svg)](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Roxy-0304/band-heart-rate)](https://github.com/Roxy-0304/band-heart-rate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

## 简介

**Band Heart Rate Monitor** 是一款基于 Rust 和 Slint 的原生桌面心率监测应用，通过标准 BLE 心率服务（UUID 0x180D）接收穿戴设备的实时心率数据。内置 HTTP 服务器，支持 REST 和 SSE 实时推送，便于集成到直播叠加层。

需要在手环/手表的设置中开启心率广播功能。

> 💡 最新版本可从 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载。

## 功能特性

- **Slint 原生窗口** — CPU 和内存占用极低，高帧率刷新
- **实时心率显示** — 大数字实时刷新，自动识别热身/燃脂/有氧/极限四区
- **实时统计** — 最低/最高/平均心率，支持一键重置
- **设置面板** — 可配置最大心率、设备关键词、HTTP 端口、开机自启、最小化到托盘
- **HTTP API** — REST 接口 + SSE 实时推送 + 配置读写接口
- **自动重连** — 断开后自动扫描重连，指数退避
- **原生 Windows 支持** — macOS / Linux 需自行添加平台依赖

## 快速开始

### 下载

前往 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载对应平台的可执行文件，直接运行即可。

### 源码编译

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# 带 GUI 版本
cargo build --release

# 纯后端版本（仅 HTTP API）
cargo build --release --no-default-features
```

**环境要求：** [Rust 工具链](https://www.rust-lang.org/tools/install)（推荐 rustup）

> 💡 **macOS / Linux 用户注意：** GUI 版本主要面向 Windows。非 GUI 模式（`cargo build --release --no-default-features`）在各平台均可编译。若在 macOS/Linux 上编译 GUI 版本，需自行安装平台相关系统依赖（如 `libdbus-1-dev`、`pkg-config`、`libwebkit2gtk-4.1-dev` 等）。

## 使用指南

1. 在手环/手表设置中开启 **心率广播**
2. 确保设备蓝牙已开启
3. 运行程序，自动扫描并连接心率设备
4. 心率数据在原生窗口实时显示，同时通过 HTTP API 提供访问
5. 点击右上角齿轮图标打开设置面板，按需调整配置

## HTTP API

| 接口 | 说明 |
|------|------|
| `GET /heart-rate` | 当前心率 JSON |
| `GET /heart-rate-stream` | SSE 实时心率数据流 |
| `GET /settings` | 获取当前配置 |
| `PUT /settings` | 更新配置（JSON body） |
| `GET /health` | 健康检查 |

默认地址 `http://127.0.0.1:3030`，端口可在设置面板中修改，端口冲突时自动使用随机端口。

## 配置

配置保存在系统配置目录（Windows 为 `%APPDATA%/band-heart-rate/settings.json`），也可通过设置面板或 HTTP API 修改。

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `max_heart_rate` | 最大心率（影响区间划分） | `190` |
| `allowed_devices` | 允许连接的设备关键词（逗号分隔） | `band,amazfit,watch,mi` |
| `server_port` | HTTP 服务端口 | `3030` |
| `auto_start` | 开机自启动 | `false` |
| `minimize_to_tray` | 关闭时最小化到托盘 | `true` |

## 兼容设备

兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备。GUI 版本主要在 Windows 10/11 上测试，非 GUI 模式支持所有平台。

支持的设备包括：小米手环、荣耀手环、华为手环/手表、Amazfit、Apple Watch 等。在设备设置中开启"心率广播"即可被识别。

## 截图

**Web 界面**

![Web 界面](doc/1.png)

**主界面**

![主界面](doc/2.png)

**设置面板**

![设置面板](doc/3.png)

## License

[MIT](LICENSE)
