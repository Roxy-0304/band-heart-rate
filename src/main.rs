#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ble;
mod config;
mod control_server;
mod control_ui;
mod macros;
mod server;
mod tray;
mod types;
mod web_ui;

#[cfg(all(windows, feature = "tray"))]
fn attach_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has_console = args.iter().any(|a| a == "--console");

    if has_console {
        #[cfg(all(windows, feature = "tray"))]
        attach_console();
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_ansi(true)
        .init();

    tracing::info!("Band Heart Rate v{}", env!("CARGO_PKG_VERSION"));

    use std::sync::{Arc, Mutex};
    use tokio::sync::{mpsc, watch};
    use types::HeartRateReading;

    let config_manager = config::ConfigManager::new();
    let web_port = config_manager.rx.borrow().server_port;
    let control_port = web_port.wrapping_add(1).max(1);

    let (tx, rx_ble) = watch::channel(HeartRateReading::default());

    let rx_server = rx_ble.clone();
    let config_tx_server = config_manager.tx.clone();
    let config_rx_server = config_manager.rx.clone();
    let config_rx_ble = config_manager.rx.clone();

    // BLE 命令通道
    let (ble_cmd_tx, ble_cmd_rx) = mpsc::channel::<types::BleCommand>(16);
    // 设备发现共享状态
    let discovered: Arc<Mutex<Vec<types::DiscoveredDevice>>> = Arc::new(Mutex::new(Vec::new()));

    // 控制服务器状态
    let control_state = types::ControlState {
        rx: rx_ble.clone(),
        config_rx: config_manager.rx.clone(),
        config_tx: config_manager.tx.clone(),
        discovered: Arc::clone(&discovered),
        ble_cmd_tx: ble_cmd_tx.clone(),
    };

    // 后台线程：BLE + Web Server + Control Server
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            let server_handle = tokio::spawn(async move {
                if let Err(err) =
                    server::run_server(rx_server, config_tx_server, config_rx_server).await
                {
                    tracing::error!("Web 服务器错误: {err}");
                }
            });

            let control_state_inner = control_state;
            let control_handle = tokio::spawn(async move {
                if let Err(err) =
                    control_server::run_control_server(control_state_inner, control_port).await
                {
                    tracing::error!("Control 服务器错误: {err}");
                }
            });

            let adapter = match bluest::Adapter::default().await {
                Some(a) => a,
                None => {
                    tracing::error!("蓝牙适配器未找到（系统无蓝牙或驱动异常）");
                    tx.send_replace(HeartRateReading {
                        error: Some("蓝牙适配器未找到".into()),
                        ..Default::default()
                    });
                    let _ = server_handle.await;
                    let _ = control_handle.await;
                    return;
                }
            };

            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                adapter.wait_available(),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!("蓝牙适配器不可用: {e}");
                    tx.send_replace(HeartRateReading {
                        error: Some(format!("蓝牙适配器不可用: {e}")),
                        ..Default::default()
                    });
                    let _ = server_handle.await;
                    let _ = control_handle.await;
                    return;
                }
                Err(_) => {
                    tracing::error!("蓝牙适配器无响应（5 秒超时），请检查蓝牙是否开启");
                    tx.send_replace(HeartRateReading {
                        error: Some("蓝牙适配器无响应，请检查蓝牙是否开启".into()),
                        ..Default::default()
                    });
                    let _ = server_handle.await;
                    let _ = control_handle.await;
                    return;
                }
            }

            if let Err(e) =
                ble::run_loop(adapter, tx.clone(), config_rx_ble, discovered, ble_cmd_rx).await
            {
                tracing::error!("蓝牙循环退出: {e}");
                tx.send_replace(HeartRateReading {
                    error: Some(format!("蓝牙服务已停止: {e}")),
                    ..Default::default()
                });
            }

            let _ = server_handle.await;
            let _ = control_handle.await;
        });
    });

    // 主线程：系统托盘事件循环
    if let Err(e) = tray::run(rx_ble, web_port, control_port) {
        tracing::error!("托盘退出: {e}");
    }
}
