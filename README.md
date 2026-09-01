# Band Heart Rate Monitor

> 轻量级系统托盘心率监测应用，通过 BLE 接收穿戴设备实时心率，支持 OBS 直播叠加。

🌐 [English](README_EN.md) | 简体中文

> ⚠️ 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

## 📖 简介

**Band Heart Rate Monitor** 是一款基于 Rust 的轻量级系统托盘应用，通过标准 BLE 心率服务（UUID 0x180D）接收穿戴设备的实时心率数据。内置 HTTP 服务器，支持 REST 和 SSE 实时推送，轻松集成到直播叠加层（OBS）。

## ✨ 功能特性

### 🎯 核心功能

| 功能 | 说明 |
|------|------|
| 系统托盘运行 | 默认静默启动到托盘，无窗口无终端，资源占用极低 |
| 实时心率显示 | Web 界面大数字实时刷新，心形 SVG 动画 |
| Windows 通知 | 扫描中、已连接、断开、超时、错误等状态实时通知 |
| 托盘菜单 | 左键打开 Web UI，右键菜单：打开 Web UI / 复制地址 / 退出 |
| 自动重连 | 断开后自动扫描重连，指数退避 |
| 控制面板 | 独立 Web 控制面板，支持设备管理、配置修改、手动重连 |
| 重新扫描 | 扫描超时后可通过控制面板一键重新扫描，无需重启应用 |

### 🔗 集成能力

| 功能 | 说明 |
|------|------|
| OBS 覆盖层 | `overlay` 模式提供透明背景的心率叠加层 |
| HTTP API | REST 接口 + SSE 实时推送 + 配置读写接口 |

### 🛠️ 开发者功能

| 功能 | 说明 |
|------|------|
| 配置文件 | JSON 格式配置，支持 HTTP API 读写 |
| 跨平台支持 | 非托盘模式各平台均可编译 |

## 🚀 快速开始

### 方式一：下载安装（推荐）

1. 前往 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载最新版本
2. 直接运行，无需安装

- **默认启动**：静默启动到系统托盘，无窗口
- **`--console` 参数**：显示终端窗口，输出日志信息

### 方式二：源码编译

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# 完整版本（系统托盘 + BLE + Web 服务器）
cargo build --release

# 纯后端版本（仅 HTTP API，无 BLE 和托盘）
cargo build --release --no-default-features
```

### 环境要求

- **操作系统**：Windows（完整版）/ macOS / Linux（非托盘模式）
- **Rust 版本**：Edition 2024（推荐通过 [rustup](https://www.rust-lang.org/tools/install) 安装）

> 💡 macOS / Linux 用户注意：托盘版本主要面向 Windows。非托盘模式（`--no-default-features`）各平台均可编译。若在 macOS/Linux 上编译托盘版本，需自行安装平台相关系统依赖。

## 📖 使用指南

### 基本使用

1. 在手环/手表设置中开启 **心率广播**
2. 确保设备蓝牙已开启
3. 运行程序，自动扫描并连接心率设备
4. 托盘图标实时显示心率，状态变化通过 Windows 通知提醒
5. 左键托盘图标打开 Web UI，右键打开菜单

### 托盘操作

| 操作 | 说明 |
|------|------|
| 左键托盘图标 | 打开 Web UI |
| 右键托盘图标 | 菜单：打开 Web UI / 复制地址 / 退出 |

### 系统通知

| 通知 | 触发时机 |
|------|----------|
| 扫描中 | 开始扫描蓝牙设备 |
| 已连接 | 成功连接到设备 |
| 断开 | 设备断开连接，正在重连 |
| 超时 | 扫描超时未找到设备 |
| 错误 | 蓝牙适配器异常等错误 |

## ⚙️ 配置

配置保存在系统配置目录：

- **Windows**：`%APPDATA%/band-heart-rate/settings.json`
- 也可通过 HTTP API 修改

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `max_heart_rate` | 最大心率（影响区间划分） | `190` |
| `allowed_devices` | 允许连接的设备关键词（逗号分隔） | `band,amazfit,watch,mi` |
| `server_port` | HTTP 服务端口 | `3030` |
| `auto_start` | 开机自启动 | `false` |
| `minimize_to_tray` | 关闭时最小化到托盘 | `true` |

## 🔌 API 参考

默认地址 `http://127.0.0.1:3030`，端口可在配置中修改，端口冲突时自动使用随机端口。

### Web UI 端口（默认 3030）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | Web UI 页面（实时心率显示） |
| GET | `/heart-rate` | 当前心率 JSON |
| GET | `/heart-rate-stream` | SSE 实时心率数据流 |
| GET | `/health` | 健康检查 |

### 控制面板端口（默认 3031 = Web 端口 + 1）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 控制面板页面 |
| GET | `/heart-rate` | 当前心率 JSON |
| GET | `/devices` | 已发现的设备列表 |
| POST | `/devices/select` | 选择连接设备（JSON body: `{"device_id": "..."}`） |
| POST | `/devices/disconnect` | 断开当前连接 |
| POST | `/devices/rescan` | 重新扫描（重置超时状态，重新开始扫描） |
| GET | `/settings` | 获取当前配置 |
| PUT | `/settings` | 更新配置（JSON body） |

## 📱 兼容设备

兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备：

- 小米手环 ✅（已测试）
- 荣耀手环 ❓
- 华为手环/手表 ❓
- Amazfit ❓
- Apple Watch ❓

> 在设备设置中开启心率广播即可被识别。❓ 表示理论兼容但未实际测试。

## ⚠️ 注意事项

- 需要在手环/手表设置中开启心率广播功能
- macOS / Linux 编译托盘版本需自行安装平台依赖
- 非托盘模式（`--no-default-features`）不包含 BLE 和系统托盘功能

## 🙏 致谢

- [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate) — 原始项目

## 📸 截图

**终端日志**

![终端日志](doc/1.png)

**控制面板**

![控制面板](doc/2.png)

## 📄 License

[MIT](LICENSE)
