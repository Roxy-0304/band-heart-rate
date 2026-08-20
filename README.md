[English](README_EN.md) | [中文](README.md)

[![CI](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml/badge.svg)](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Roxy-0304/band-heart-rate)](https://github.com/Roxy-0304/band-heart-rate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

## 简介

**Band Heart Rate Monitor** 是一款基于 Rust 的轻量级系统托盘心率监测应用，通过标准 BLE 心率服务（UUID 0x180D）接收穿戴设备的实时心率数据。内置 HTTP 服务器，支持 REST 和 SSE 实时推送，便于集成到直播叠加层（OBS）。

需要在手环/手表的设置中开启心率广播功能。

> 最新版本可从 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载。

## 功能特性

- **系统托盘运行** — 默认静默启动到托盘，无窗口无终端，资源占用极低
- **Windows 通知** — 扫描中、已连接、断开、超时、错误等状态变化实时通知
- **托盘菜单** — 左键打开 Web UI，右键菜单：打开 Web UI / 复制地址 / 退出
- **实时心率显示** — Web 界面大数字实时刷新，心形 SVG 动画
- **OBS 覆盖层** — `overlay` 模式提供透明背景的心率叠加层
- **HTTP API** — REST 接口 + SSE 实时推送 + 配置读写接口
- **自动重连** — 断开后自动扫描重连，指数退避
- **原生 Windows 支持** — macOS / Linux 需自行添加平台依赖

## 快速开始

### 下载

前往 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载可执行文件，直接运行即可。

- **默认启动**：静默启动到系统托盘，无窗口
- **`--console` 参数**：显示终端窗口，输出日志信息

### 源码编译

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# 完整版本（系统托盘 + BLE + Web 服务器）
cargo build --release

# 纯后端版本（仅 HTTP API，无 BLE 和托盘）
cargo build --release --no-default-features
```

**环境要求：** [Rust 工具链](https://www.rust-lang.org/tools/install)（推荐 rustup）

> **macOS / Linux 用户注意：** 托盘版本主要面向 Windows。非托盘模式（`cargo build --release --no-default-features`）在各平台均可编译。若在 macOS/Linux 上编译托盘版本，需自行安装平台相关系统依赖。

## 使用指南

1. 在手环/手表设置中开启 **心率广播**
2. 确保设备蓝牙已开启
3. 运行程序，自动扫描并连接心率设备
4. 托盘图标实时显示心率，状态变化通过 Windows 通知提醒
5. 左键托盘图标打开 Web UI，右键打开菜单

## 托盘菜单

| 菜单项 | 功能 |
|--------|------|
| 打开 Web UI | 在浏览器中打开心率监测页面 |
| 复制地址 | 复制 Web UI 地址到剪贴板 |
| 退出 | 退出应用 |

## 系统通知

| 通知 | 触发时机 |
|------|----------|
| 扫描中 | 开始扫描蓝牙设备 |
| 已连接 | 成功连接到设备 |
| 断开 | 设备断开连接，正在重连 |
| 超时 | 扫描超时未找到设备 |
| 错误 | 蓝牙适配器异常等错误 |

## HTTP API

| 接口 | 说明 |
|------|------|
| `GET /` | Web UI 页面 |
| `GET /heart-rate` | 当前心率 JSON |
| `GET /heart-rate-stream` | SSE 实时心率数据流 |
| `GET /settings` | 获取当前配置 |
| `PUT /settings` | 更新配置（JSON body） |
| `GET /health` | 健康检查 |

默认地址 `http://127.0.0.1:3030`，端口可在配置文件中修改，端口冲突时自动使用随机端口。

## 配置

配置保存在系统配置目录（Windows 为 `%APPDATA%/band-heart-rate/settings.json`），也可通过 HTTP API 修改。

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `max_heart_rate` | 最大心率（影响区间划分） | `190` |
| `allowed_devices` | 允许连接的设备关键词（逗号分隔） | `band,amazfit,watch,mi` |
| `server_port` | HTTP 服务端口 | `3030` |
| `auto_start` | 开机自启动 | `false` |
| `minimize_to_tray` | 关闭时最小化到托盘 | `true` |

## 兼容设备

兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备。

支持的设备包括：小米手环、荣耀手环、华为手环/手表、Amazfit、Apple Watch 等。在设备设置中开启心率广播即可被识别。

## 截图

**终端日志**

![终端日志](doc/1.png)

**控制面板**

![控制面板](doc/2.png)

## License

[MIT](LICENSE)