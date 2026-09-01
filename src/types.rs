use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

/// 设备连接/通信过程中发生的可重连错误类型
pub enum ReconnectError {
    /// 设备已物理断开连接
    Disconnected,
    /// 设备仍连接但停止广播数据（如手环退出心率模式）
    StoppedBroadcasting,
}

impl fmt::Display for ReconnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReconnectError::Disconnected => write!(f, "Device disconnected"),
            ReconnectError::StoppedBroadcasting => write!(f, "Device stopped broadcasting"),
        }
    }
}

impl fmt::Debug for ReconnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ReconnectError {}

#[derive(Clone, Default, Serialize)]
pub struct HeartRateReading {
    pub heart_rate: u16,
    pub sensor_contact: Option<bool>,
    pub connected: bool,
    pub scanning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub name: String,
    pub rssi: Option<i16>,
}

/// BLE 层发给控制服务器的命令
pub enum BleCommand {
    /// 手动选择连接某个设备
    SelectDevice(String),
    /// 断开当前连接
    Disconnect,
    /// 重新扫描（重置超时状态，重新开始扫描）
    Rescan,
}

/// 全局共享状态
pub struct ControlState {
    pub rx: watch::Receiver<HeartRateReading>,
    pub config_rx: watch::Receiver<crate::config::Config>,
    pub config_tx: watch::Sender<crate::config::Config>,
    pub discovered: Arc<Mutex<Vec<DiscoveredDevice>>>,
    pub ble_cmd_tx: mpsc::Sender<BleCommand>,
}

impl Clone for ControlState {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
            config_rx: self.config_rx.clone(),
            config_tx: self.config_tx.clone(),
            discovered: Arc::clone(&self.discovered),
            ble_cmd_tx: self.ble_cmd_tx.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub rx: watch::Receiver<HeartRateReading>,
    pub config_rx: watch::Receiver<crate::config::Config>,
    pub config_tx: watch::Sender<crate::config::Config>,
}
