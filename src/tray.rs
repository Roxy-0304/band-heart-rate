use std::sync::mpsc;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};

use crate::types::HeartRateReading;

const MENU_OPEN: &str = "open";
const MENU_COPY: &str = "copy";
const MENU_QUIT: &str = "quit";

/// Send a Windows notification via notify-rust
fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .appname("Band Heart Rate")
        .summary(title)
        .body(body)
        .show();
}

/// Load tray icon from embedded PNG
fn load_tray_icon() -> anyhow::Result<tray_icon::Icon> {
    let rgba = image::load_from_memory(include_bytes!("../icons/icon.png"))
        .map_err(|e| anyhow::anyhow!("Failed to load tray icon: {e}"))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(tray_icon::Icon::from_rgba(rgba.into_raw(), width, height)?)
}

/// Build the tooltip string from heart rate reading
fn build_tooltip(reading: &HeartRateReading, lang: &str) -> String {
    use crate::i18n::{t, tf};
    if let Some(ref err) = reading.error {
        format!("❌ {err}")
    } else if reading.scanning {
        t(lang, "tip_scanning").to_string()
    } else if reading.connected && reading.heart_rate > 0 {
        let name = reading.device_name.as_deref().unwrap_or(if lang == "zh" {
            "未知设备"
        } else {
            "Unknown"
        });
        tf(
            lang,
            "tip_connected",
            &[&reading.heart_rate.to_string(), name],
        )
    } else if reading.connected {
        let name = reading.device_name.as_deref().unwrap_or(if lang == "zh" {
            "未知设备"
        } else {
            "Unknown"
        });
        tf(lang, "tip_connected_no_hr", &[name])
    } else {
        t(lang, "tip_disconnected").to_string()
    }
}

/// Run the tray application on the main thread.
pub fn run(
    rx: tokio::sync::watch::Receiver<HeartRateReading>,
    web_port: u16,
    control_port: u16,
    config_rx: tokio::sync::watch::Receiver<crate::config::Config>,
) -> anyhow::Result<()> {
    use tao::platform::windows::EventLoopBuilderExtWindows;
    let mut event_loop = EventLoopBuilder::new();
    event_loop.with_any_thread(false);
    let event_loop = event_loop.build();

    let (cmd_tx, cmd_rx) = mpsc::channel::<String>();

    // --- Set up tray icon ---
    let icon = load_tray_icon()?;
    let menu = muda::Menu::new();
    let open_item = muda::MenuItem::with_id(
        MENU_OPEN,
        "打开控制面板",
        true,
        None::<muda::accelerator::Accelerator>,
    );
    let quit_item = muda::MenuItem::with_id(
        MENU_QUIT,
        "退出",
        true,
        None::<muda::accelerator::Accelerator>,
    );
    let copy_item = muda::MenuItem::with_id(
        MENU_COPY,
        "复制地址",
        true,
        None::<muda::accelerator::Accelerator>,
    );
    let _ = menu.append(&open_item);
    let _ = menu.append(&copy_item);
    let _ = menu.append(&quit_item);

    // 根据初始语言设置菜单文本（避免首次启动时始终显示中文）
    {
        use crate::i18n::t;
        let lang = config_rx.borrow().language.clone();
        open_item.set_text(t(&lang, "menu_open"));
        copy_item.set_text(t(&lang, "menu_copy"));
        quit_item.set_text(t(&lang, "menu_quit"));
    }

    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("Band Heart Rate Monitor")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()?;

    // 后台线程：监听心率变化 → 更新 tooltip + 直接发通知（不经主线程，避免延迟）
    {
        let cmd_tx = cmd_tx.clone();
        let mut rx = rx.clone();
        let config_rx = config_rx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                let mut last_tooltip = String::new();
                let mut was_connected = false;
                let mut was_scanning = false;
                let mut last_error: Option<String> = None;

                while rx.changed().await.is_ok() {
                    let reading = rx.borrow().clone();
                    let lang = config_rx.borrow().language.clone();

                    // --- 更新 tooltip（需要主线程处理） ---
                    let tooltip = build_tooltip(&reading, &lang);
                    if tooltip != last_tooltip {
                        last_tooltip = tooltip.clone();
                        let _ = cmd_tx.send(tooltip);
                    }

                    // --- 发送通知（直接在后台线程发，不经过主线程消息泵） ---
                    use crate::i18n::t;

                    // 1. 开始搜索：scanning false→true
                    if reading.scanning && !was_scanning {
                        notify(&t(&lang, "notif_scanning"), &t(&lang, "msg_scanning"));
                    }

                    // 2. 已连接：connected false→true
                    if reading.connected && !was_connected {
                        let name = reading.device_name.as_deref().unwrap_or(if lang == "zh" {
                            "设备"
                        } else {
                            "device"
                        });
                        notify(
                            &t(&lang, "notif_connected"),
                            &format!("{} {name}", t(&lang, "msg_connected")),
                        );
                    }

                    // 3. 断开：connected true→false 且无 error
                    if !reading.connected && was_connected && reading.error.is_none() {
                        notify(
                            &t(&lang, "notif_disconnected"),
                            &t(&lang, "msg_disconnected"),
                        );
                    }

                    // 4. 错误 / 超时：error 变化
                    if let Some(ref err) = reading.error {
                        if last_error.as_ref() != Some(err) {
                            if err.contains("超时") || err.contains("timeout") {
                                notify(&t(&lang, "notif_timeout"), err);
                            } else {
                                notify(&t(&lang, "notif_error"), err);
                            }
                        }
                    }

                    was_connected = reading.connected;
                    was_scanning = reading.scanning;
                    last_error = reading.error.clone();
                }
            });
        });
    }

    tracing::info!("系统托盘已启动");

    // 克隆菜单项引用，用于事件循环中更新文本
    let open_item = open_item.clone();
    let copy_item = copy_item.clone();
    let quit_item = quit_item.clone();
    let mut last_lang = config_rx.borrow().language.clone();

    // --- tao event loop (main thread) ---
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // 检查语言变化，更新菜单文本
        let current_lang = config_rx.borrow().language.clone();
        if current_lang != last_lang {
            last_lang = current_lang;
            use crate::i18n::t;
            open_item.set_text(t(&last_lang, "menu_open"));
            copy_item.set_text(t(&last_lang, "menu_copy"));
            quit_item.set_text(t(&last_lang, "menu_quit"));
        }

        // Process tooltip updates from background thread
        while let Ok(cmd) = cmd_rx.try_recv() {
            let tooltip = cmd;
            let _ = tray.set_tooltip(Some(&tooltip));
        }

        // Process menu events from muda
        while let Ok(menu_event) = muda::MenuEvent::receiver().try_recv() {
            if menu_event.id == MENU_OPEN {
                let url = format!("http://127.0.0.1:{control_port}");
                tracing::info!("打开浏览器: {url}");
                let _ = open::that(&url);
            } else if menu_event.id == MENU_COPY {
                let url = format!("http://127.0.0.1:{web_port}");
                #[cfg(target_os = "windows")]
                {
                    use clipboard_win::{formats::Unicode, set_clipboard};
                    let _ = set_clipboard(Unicode, &url);
                }
                tracing::info!("已复制地址: {url}");
            } else if menu_event.id == MENU_QUIT {
                tracing::info!("用户选择退出");
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::MainEventsCleared => {}
            Event::RedrawRequested(_) => {}
            _ => {}
        }
    });
}
