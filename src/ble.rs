use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use bluest::{Adapter, Device, Uuid, btuuid::bluetooth_uuid_from_u16};
use futures_lite::stream::StreamExt;
use tokio::sync::watch;
use tokio::time::{Duration, timeout};

use crate::config::Config;
use crate::macros::printfl_inline;
use crate::types::{BleCommand, DiscoveredDevice, HeartRateReading, ReconnectError};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const HRS_UUID: Uuid = bluetooth_uuid_from_u16(0x180D);
const HRM_UUID: Uuid = bluetooth_uuid_from_u16(0x2A37);

/// 扫描总超时：2 分钟无设备则退出
const SCAN_TOTAL_TIMEOUT_SECS: u64 = 120;
/// 单次扫描超时：最多 10 秒
const SCAN_ATTEMPT_TIMEOUT_SECS: u64 = 10;
/// 首次连接后等待第一个数据的超时时间
const INITIAL_DATA_TIMEOUT_SECS: u64 = 30;
/// 正常运行时两个数据包之间的超时时间
const NORMAL_DATA_TIMEOUT_SECS: u64 = 25;
/// 重连退避最大值（秒）
const RECONNECT_BACKOFF_MAX_SECS: u64 = 30;
/// 扫描失败后重试间隔
const SCAN_RETRY_DELAY_SECS: u64 = 1;

/// 默认允许连接的设备名称关键词（不区分大小写，名称包含任意一个即匹配）
const DEFAULT_ALLOWED_KEYWORDS: &[&str] = &["band", "amazfit", "watch", "mi"];

/// 从配置中解析允许的设备名称关键词
fn allowed_keywords(config: &Config) -> Vec<String> {
    let keywords: Vec<String> = config
        .allowed_devices
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if keywords.is_empty() {
        DEFAULT_ALLOWED_KEYWORDS
            .iter()
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        keywords
    }
}

/// 检查设备名称是否匹配允许的关键词列表。
/// 没有名称的设备视为不匹配（未知设备不应盲目连接），
/// 已在 known_ids 中的设备由调用方绕过此检查。
fn device_name_allowed(name: Option<&str>, keywords: &[String]) -> bool {
    match name {
        Some(name) => {
            let name_lower = name.to_lowercase();
            keywords.iter().any(|kw| name_lower.contains(kw.as_str()))
        }
        None => false,
    }
}

/// 根据重连尝试次数计算指数退避秒数
fn compute_backoff(attempts: u32) -> u64 {
    std::cmp::min(1u64 << attempts.min(10), RECONNECT_BACKOFF_MAX_SECS)
}

// ===== BleSession: 封装蓝牙连接/重连状态 =====

struct BleSession {
    known_ids: HashSet<String>,
    reconnect_attempts: u32,
    has_ever_connected: bool,
    disconnect_time: Option<Instant>,
}

impl BleSession {
    fn new() -> Self {
        Self {
            known_ids: HashSet::new(),
            reconnect_attempts: 0,
            has_ever_connected: false,
            disconnect_time: None,
        }
    }

    /// 标记设备连接成功
    fn mark_connected(&mut self, device_id: String) {
        self.known_ids.insert(device_id);
        self.has_ever_connected = true;
        self.disconnect_time = None;
    }

    /// 标记设备断开，准备重连
    fn mark_disconnected(&mut self, device_id: String) {
        self.known_ids.insert(device_id);
        self.has_ever_connected = true;
        self.disconnect_time = Some(Instant::now());
    }

    /// 是否处于重连模式（曾经连接成功但现在断开了）
    fn is_reconnecting(&self) -> bool {
        self.has_ever_connected && self.disconnect_time.is_some()
    }

    /// 检查是否已超过总超时时间，返回 Ok(()) 或 Err
    fn check_timeout(&self) -> anyhow::Result<()> {
        if let Some(time) = self.disconnect_time {
            let elapsed = time.elapsed().as_secs();
            if elapsed >= SCAN_TOTAL_TIMEOUT_SECS {
                anyhow::bail!("扫描超时：120 秒内未找到任何设备");
            }
        }
        Ok(())
    }

    /// 计算当前退避秒数
    fn backoff_secs(&self) -> u64 {
        compute_backoff(self.reconnect_attempts)
    }
}

// ===== 公开入口 =====

/// 蓝牙主循环：扫描、连接、重连
pub async fn run_loop(
    adapter: Adapter,
    tx: watch::Sender<HeartRateReading>,
    config_rx: watch::Receiver<Config>,
    discovered: Arc<Mutex<Vec<DiscoveredDevice>>>,
    mut ble_cmd_rx: mpsc::Receiver<BleCommand>,
    cancel_flag: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut session = BleSession::new();

    loop {
        // 检查是否有手动选择命令
        while let Ok(cmd) = ble_cmd_rx.try_recv() {
            match cmd {
                BleCommand::SelectDevice(device_id) => {
                    tracing::info!("手动选择连接设备: {device_id}");
                    // 扫描找设备并连接
                    match try_connect_by_id(&adapter, &device_id, tx.clone()).await {
                        Ok(()) => {
                            session.mark_connected(device_id);
                            continue;
                        }
                        Err(e) => {
                            tracing::warn!("手动连接失败: {e}");
                        }
                    }
                }
                BleCommand::Disconnect => {
                    tracing::info!("收到断开命令，断开当前连接");
                    cancel_flag.store(true, Ordering::Relaxed);
                    tx.send_replace(HeartRateReading::default());
                    session.has_ever_connected = false;
                    session.disconnect_time = None;
                    session.reconnect_attempts = 0;
                    session.known_ids.clear();
                }
                BleCommand::Rescan => {
                    tracing::info!("收到重新扫描命令，重置会话状态");
                    cancel_flag.store(true, Ordering::Relaxed);
                    tx.send_replace(HeartRateReading::default());
                    // 重置会话状态，允许重新扫描
                    session.has_ever_connected = false;
                    session.disconnect_time = None;
                    session.reconnect_attempts = 0;
                    session.known_ids.clear();
                    // 跳过本次超时检查，直接继续循环
                    continue;
                }
            }
        }

        session.check_timeout()?;
        let config = config_rx.borrow().clone();

        let is_reconnecting = session.is_reconnecting();
        tx.send_replace(HeartRateReading {
            scanning: true,
            ..Default::default()
        });

        if is_reconnecting {
            session.reconnect_attempts += 1;
            printfl_inline!("[重连 #{}] 正在扫描设备...", session.reconnect_attempts);
        } else {
            session.reconnect_attempts = 0;
            tracing::info!("正在扫描设备...");
        }

        // 扫描设备
        let devices =
            match scan_all_devices(&adapter, tx.clone(), is_reconnecting, &discovered).await {
                Ok(devices) => devices,
                Err(err) => {
                    if is_reconnecting {
                        printfl_inline!(
                            "[重连 #{}] 扫描失败，等待重试...",
                            session.reconnect_attempts
                        );
                    } else {
                        tracing::warn!("扫描失败: {err:?}");
                    }
                    tx.send_replace(HeartRateReading::default());
                    tokio::time::sleep(Duration::from_secs(SCAN_RETRY_DELAY_SECS)).await;
                    continue;
                }
            };

        // 尝试连接每个设备 — known_ids 优先
        let keywords = allowed_keywords(&config);
        let (known_devices, other_devices): (Vec<&Device>, Vec<&Device>) = devices
            .iter()
            .partition(|d| session.known_ids.contains(&d.id().to_string()));

        let mut connected_this_round = false;
        for device in known_devices.iter().chain(other_devices.iter()) {
            let device_id = device.id().to_string();
            let device_name = device.name_async().await.ok();
            if !session.known_ids.contains(&device_id)
                && !device_name_allowed(device_name.as_deref(), &keywords)
            {
                if !is_reconnecting {
                    tracing::debug!(
                        "跳过设备 [{}] {:?}: 不在允许关键词列表中",
                        device_id,
                        device_name
                    );
                }
                continue;
            }
            if is_reconnecting {
                printfl_inline!(
                    "[重连 #{}] 正在连接设备 {}...",
                    session.reconnect_attempts,
                    device_id
                );
            }
            match handle_device(&adapter, device, tx.clone(), Some(cancel_flag.clone())).await {
                Ok(()) => {
                    session.mark_connected(device_id);
                    connected_this_round = true;
                    break;
                }
                Err(err) => {
                    if err.downcast_ref::<ReconnectError>().is_some() {
                        session.mark_disconnected(device_id);
                        connected_this_round = true;
                        continue;
                    }
                    if !is_reconnecting {
                        tracing::warn!("无法连接该设备: {err:?}，尝试下一个...");
                    }
                    continue;
                }
            }
        }

        if connected_this_round && session.disconnect_time.is_none() {
            // 刚连接成功，立即回到循环顶部继续接收数据
            continue;
        }

        // 所有设备已穷尽
        tx.send_replace(HeartRateReading::default());
        if session.disconnect_time.is_none() {
            session.disconnect_time = Some(Instant::now());
        }

        if session.has_ever_connected {
            let backoff_secs = session.backoff_secs();
            let detail = if devices.is_empty() {
                "未找到可连接设备"
            } else {
                "等待重试"
            };
            printfl_inline!(
                "[重连 #{}] {detail} {backoff_secs} 秒后重扫...",
                session.reconnect_attempts
            );
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        } else {
            // 从未连接 — 进入轻量轮询
            session.reconnect_attempts = 0;
            tracing::info!("所有设备忙碌或不可达，进入轻量轮询模式...");

            match poll_for_available_device(&adapter, tx.clone(), &session.known_ids, &config_rx)
                .await
            {
                Ok(Some(device_id)) => {
                    // 轮询成功连接了设备（设备已断开），标记为已知设备
                    session.mark_connected(device_id);
                    session.disconnect_time = None;
                }
                Ok(None) => {
                    session.disconnect_time = None;
                }
                Err(e) => {
                    return Err(e);
                }
            }
            continue;
        }
    }
}

// ===== 扫描 =====

/// 扫描并收集所有发现的设备（最多 SCAN_ATTEMPT_TIMEOUT_SECS 秒）。
/// 一旦发现第一个匹配的设备就进入 settle 模式：再等 3 秒收集备选，然后返回。
async fn scan_all_devices(
    adapter: &Adapter,
    tx: watch::Sender<HeartRateReading>,
    is_reconnecting: bool,
    discovered: &Arc<Mutex<Vec<DiscoveredDevice>>>,
) -> anyhow::Result<Vec<Device>> {
    tx.send_replace(HeartRateReading {
        scanning: true,
        ..Default::default()
    });

    let mut scan = match tokio::time::timeout(
        Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS),
        adapter.discover_devices(&[HRS_UUID]),
    )
    .await
    {
        Ok(Ok(scan)) => scan,
        Ok(Err(e)) => anyhow::bail!("蓝牙扫描启动失败: {e}"),
        Err(_) => anyhow::bail!("蓝牙扫描启动超时（10 秒无响应）"),
    };

    let mut seen_ids: HashSet<String> = HashSet::new();
    // 设备及其名称一起存储，避免后续重复异步获取
    let mut devices: Vec<(Device, String)> = Vec::new();

    let overall_deadline = Instant::now() + Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS);
    let settle_duration = Duration::from_secs(3);
    let mut settle_deadline: Option<Instant> = None;

    loop {
        let now = Instant::now();

        if let Some(sd) = settle_deadline
            && now >= sd
        {
            break;
        }
        if now >= overall_deadline {
            break;
        }

        let remaining = overall_deadline.duration_since(now);
        let wait_timeout = if let Some(sd) = settle_deadline {
            std::cmp::min(sd.duration_since(now), remaining)
        } else {
            remaining
        };

        match tokio::time::timeout(wait_timeout, scan.next()).await {
            Ok(Some(Ok(device))) => {
                let name = device.name_async().await.unwrap_or_default().to_string();
                let did = device.id().to_string();
                if seen_ids.insert(did.clone()) {
                    devices.push((device, name.clone()));
                    if !is_reconnecting {
                        tracing::info!("发现设备: [{did}] {name:?}");
                    }
                    if settle_deadline.is_none() {
                        settle_deadline = Some(Instant::now() + settle_duration);
                        if !is_reconnecting {
                            tracing::info!("已找到首个设备，继续收集 3 秒备选...");
                        }
                    }
                }
            }
            Ok(Some(Err(e))) => {
                if !is_reconnecting {
                    tracing::warn!("扫描错误: {e:?}");
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    tx.send_replace(HeartRateReading::default());

    // 发布发现的设备到共享状态（名称在扫描阶段已获取，无需再次异步调用）
    {
        let mut list = discovered.lock().unwrap();
        list.clear();
        for (d, name) in &devices {
            list.push(DiscoveredDevice {
                id: d.id().to_string(),
                name: name.clone(),
                rssi: None,
            });
        }
    }

    if devices.is_empty() {
        anyhow::bail!("未找到任何设备");
    }
    if !is_reconnecting {
        tracing::info!("扫描完成，发现 {} 个设备", devices.len());
    }
    Ok(devices.into_iter().map(|(d, _)| d).collect())
}

// ===== 轻量轮询 =====

/// 全部设备被占时，用较短的扫描快速轮询是否有人释放
/// 返回 Ok(Some(device_id)) 表示成功连接了某个设备（已断开），Ok(None) 不会发生，
/// Err 表示超时退出。
async fn poll_for_available_device(
    adapter: &Adapter,
    tx: watch::Sender<HeartRateReading>,
    known_ids: &HashSet<String>,
    config_rx: &watch::Receiver<Config>,
) -> anyhow::Result<Option<String>> {
    let deadline = Duration::from_secs(SCAN_TOTAL_TIMEOUT_SECS);
    let poll_duration = Duration::from_secs(5);
    let start = Instant::now();
    let mut poll_count: u32 = 0;

    loop {
        poll_count += 1;
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            anyhow::bail!("轮询超时：所有设备均忙碌，退出程序");
        }

        printfl_inline!(
            "[轮询 #{poll_count}] 正在扫描可用设备（剩余 {} 秒）...",
            deadline.as_secs().saturating_sub(elapsed.as_secs())
        );

        tx.send_replace(HeartRateReading {
            scanning: true,
            ..Default::default()
        });

        let mut scan = match tokio::time::timeout(
            Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS),
            adapter.discover_devices(&[HRS_UUID]),
        )
        .await
        {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                printfl_inline!("[轮询 #{poll_count}] 扫描错误: {e:?}");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(_) => {
                printfl_inline!("[轮询 #{poll_count}] 扫描启动超时");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut devices: Vec<Device> = Vec::new();
        let _ = timeout(poll_duration, async {
            while let Some(device_result) = scan.next().await {
                if let Ok(device) = device_result {
                    devices.push(device);
                }
            }
        })
        .await;

        tx.send_replace(HeartRateReading::default());

        if devices.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let keywords = allowed_keywords(&config_rx.borrow());
        let (known_devices, other_devices): (Vec<&Device>, Vec<&Device>) = devices
            .iter()
            .partition(|d| known_ids.contains(&d.id().to_string()));

        for device in known_devices.iter().chain(other_devices.iter()) {
            let device_name = device.name_async().await.ok();
            if !known_ids.contains(&device.id().to_string())
                && !device_name_allowed(device_name.as_deref(), &keywords)
            {
                continue;
            }
            printfl_inline!("[轮询 #{poll_count}] 尝试连接设备 {}...", device.id());
            match handle_device(adapter, device, tx.clone(), None).await {
                Ok(()) => {
                    let device_id = device.id().to_string();
                    tracing::info!("[✓] 轮询成功，已连接设备 {}", device_id);
                    return Ok(Some(device_id));
                }
                Err(_err) => {
                    // ReconnectError 表示设备短暂连接后断开/停止广播，不算轮询成功
                    continue;
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ===== 单设备处理 =====

/// 连接并接收单个设备的心率数据。
/// - `Ok(())` — 通知流正常结束（设备主动停止广播）
/// - `Err(ReconnectError::Disconnected)` — 设备物理断开
/// - `Err(ReconnectError::StoppedBroadcasting)` — 数据流超时中断
/// - `Err(其他)` — 连接阶段失败（从未成功连接）
async fn handle_device(
    adapter: &Adapter,
    device: &Device,
    tx: watch::Sender<HeartRateReading>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> anyhow::Result<()> {
    tracing::info!("正在连接设备: {}", device.id());

    // 先断开清理状态，再重连
    let _ = tokio::time::timeout(Duration::from_secs(3), adapter.disconnect_device(device)).await;
    adapter.connect_device(device).await?;
    tracing::info!("设备连接成功");

    // 发现服务
    tracing::info!("正在发现服务...");
    let heart_rate_services = device.discover_services_with_uuid(HRS_UUID).await?;
    let heart_rate_service = heart_rate_services
        .first()
        .ok_or_else(|| anyhow::anyhow!("设备至少应包含一个心率服务"))?;

    // 发现特征
    tracing::info!("正在发现特征...");
    let heart_rate_measurements = heart_rate_service
        .discover_characteristics_with_uuid(HRM_UUID)
        .await?;
    let heart_rate_measurement = heart_rate_measurements
        .first()
        .ok_or_else(|| anyhow::anyhow!("心率服务至少应包含一个心率测量特征"))?;

    tracing::info!("正在设置通知...");
    let mut updates = heart_rate_measurement.notify().await?;

    // 发送已连接状态
    let device_addr = device.id().to_string();
    let device_name = device.name_async().await.ok();
    let da = device_addr.clone();
    let dn = device_name.clone();
    tx.send_replace(HeartRateReading {
        connected: true,
        device_address: Some(device_addr),
        device_name,
        ..Default::default()
    });

    tracing::info!("开始接收心率数据...");

    // 接收循环
    let mut last_update_time = Instant::now();
    let mut first_data_received = false;
    let initial_timeout = Duration::from_secs(INITIAL_DATA_TIMEOUT_SECS);
    let normal_timeout = Duration::from_secs(NORMAL_DATA_TIMEOUT_SECS);

    loop {
        // 检查是否收到断开命令
        if let Some(ref flag) = cancel_flag
            && flag.load(Ordering::Relaxed)
        {
            flag.store(false, Ordering::Relaxed);
            tracing::info!("收到取消信号，立即断开");
            return cleanup_and_disconnect(adapter, device, tx, ReconnectError::Disconnected).await;
        }

        let timeout_duration = if !first_data_received {
            initial_timeout
        } else {
            normal_timeout
        };

        let elapsed = last_update_time.elapsed();
        if elapsed >= timeout_duration {
            tracing::info!(
                "已 {} 秒未收到心率数据，准备重连...",
                timeout_duration.as_secs()
            );
            break;
        }

        let result = tokio::time::timeout(timeout_duration - elapsed, updates.next()).await;
        match result {
            Ok(Some(Ok(heart_rate))) => {
                if !first_data_received {
                    first_data_received = true;
                    tracing::info!("收到首条心率数据，切换至正常超时模式");
                }

                last_update_time = Instant::now();

                if let Some(reading) = parse_heart_rate(&heart_rate) {
                    tx.send_replace(HeartRateReading {
                        heart_rate: reading.0,
                        sensor_contact: reading.1,
                        connected: true,
                        scanning: false,
                        error: None,
                        device_address: Some(da.clone()),
                        device_name: dn.clone(),
                    });

                    printfl_inline!(
                        "心率值: {}, 传感器接触: {:?}                    ",
                        reading.0,
                        reading.1
                    );
                }
            }
            Ok(Some(Err(e))) => {
                tracing::warn!("通知错误: {e:?}");
                return cleanup_and_disconnect(adapter, device, tx, ReconnectError::Disconnected)
                    .await;
            }
            Ok(None) => {
                tracing::info!("心率通知已停止");
                return cleanup_and_disconnect(
                    adapter,
                    device,
                    tx,
                    ReconnectError::StoppedBroadcasting,
                )
                .await;
            }
            Err(_) => break,
        }
    }

    // 超时退出
    if first_data_received {
        tracing::info!("设备数据流超时，准备重连...");
    } else {
        tracing::info!("连接后未收到数据，准备重连...");
    }
    cleanup_and_disconnect(adapter, device, tx, ReconnectError::StoppedBroadcasting).await
}

/// 解析 BLE Heart Rate Measurement 数据包。
/// 格式：[flags, hr_lo, [hr_hi], [rr_lo, rr_hi, ...]]
/// 返回 (heart_rate, sensor_contact) 或 None（数据包无效）
fn parse_heart_rate(data: &[u8]) -> Option<(u16, Option<bool>)> {
    if data.is_empty() {
        tracing::warn!("收到空的心率数据包");
        return None;
    }

    let flag = data[0];
    let hr_is_16bit = flag & 0b00001 != 0;
    let hr_byte_count = if hr_is_16bit { 3 } else { 2 };

    if data.len() < hr_byte_count {
        tracing::warn!("心率数据包过短: {} 字节 (需要 {hr_byte_count})", data.len());
        return None;
    }

    let heart_rate = if hr_is_16bit {
        u16::from_le_bytes([data[1], data[2]])
    } else {
        data[1] as u16
    };

    let sensor_contact = if flag & 0b00100 != 0 {
        Some(flag & 0b00010 != 0)
    } else {
        None
    };

    Some((heart_rate, sensor_contact))
}

/// 清理状态：重置发送 + 断开连接 + 返回结果
/// - StoppedBroadcasting（通知流正常结束/超时）→ Ok — 表示设备曾经成功连接
/// - Disconnected（物理断开）→ Err — 表示连接异常断开
async fn cleanup_and_disconnect(
    adapter: &Adapter,
    device: &Device,
    tx: watch::Sender<HeartRateReading>,
    err: ReconnectError,
) -> anyhow::Result<()> {
    tx.send_replace(HeartRateReading::default());
    let _ = tokio::time::timeout(Duration::from_secs(3), adapter.disconnect_device(device)).await;
    match err {
        ReconnectError::StoppedBroadcasting => Ok(()),
        ReconnectError::Disconnected => Err(err.into()),
    }
}

/// 根据设备 ID 尝试连接（手动选择）
async fn try_connect_by_id(
    adapter: &Adapter,
    target_id: &str,
    tx: watch::Sender<HeartRateReading>,
) -> anyhow::Result<()> {
    // 扫描找设备
    let mut scan = tokio::time::timeout(
        Duration::from_secs(5),
        adapter.discover_devices(&[HRS_UUID]),
    )
    .await
    .map_err(|_| anyhow::anyhow!("扫描超时"))?
    .map_err(|e| anyhow::anyhow!("扫描失败: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        if let Ok(Some(Ok(device))) = tokio::time::timeout(remaining, scan.next()).await
            && device.id().to_string() == target_id
        {
            return handle_device(adapter, &device, tx, None).await;
        }
    }
    anyhow::bail!("未找到设备 {target_id}")
}
