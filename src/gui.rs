use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tokio::sync::watch;

use crate::config::Config;
use crate::types::HeartRateReading;

// Include the Slint UI generated at build time
slint::include_modules!();

const MENU_QUIT: &str = "quit";
const ZONES: &[(f64, f64, &str, &str)] = &[
    (0.0, 0.6, "热身", "#4FC3F7"),
    (0.6, 0.7, "燃脂", "#66BB6A"),
    (0.7, 0.8, "有氧", "#FFA726"),
    (0.8, 1.0, "极限", "#EF5350"),
];

fn get_zone(hr: u16, max_hr: u16) -> (&'static str, &'static str) {
    if hr == 0 {
        return ("--", "#4A4F58");
    }
    let max = max_hr.max(1) as f64;
    let pct = hr as f64 / max;
    for &(min, max, label, color) in ZONES {
        if pct >= min && pct < max {
            return (label, color);
        }
    }
    ("极限", "#EF5350")
}

/// Blend zone color at 12% opacity over background #0a0e16
fn zone_bg_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    let r = hex
        .get(0..2)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0) as f64;
    let g = hex
        .get(2..4)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0) as f64;
    let b = hex
        .get(4..6)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0) as f64;
    let bg_r = 10.0f64;
    let bg_g = 14.0f64;
    let bg_b = 22.0f64;
    let alpha = 0.12f64;
    let out_r = (bg_r * (1.0 - alpha) + r * alpha) as u8;
    let out_g = (bg_g * (1.0 - alpha) + g * alpha) as u8;
    let out_b = (bg_b * (1.0 - alpha) + b * alpha) as u8;
    slint::Color::from_rgb_u8(out_r, out_g, out_b)
}

fn parse_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    let r = hex
        .get(0..2)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    let g = hex
        .get(2..4)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    let b = hex
        .get(4..6)
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .unwrap_or(0);
    slint::Color::from_rgb_u8(r, g, b)
}

/// Create HICON from embedded PNG data, scaled to the given size
fn create_hicon_from_png(data: &[u8], size: u32) -> *mut std::ffi::c_void {
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let img = image::load_from_memory(data).expect("Failed to load icon PNG");
        let img = img
            .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
            .to_rgba8();

        let width = img.width() as i32;
        let height = img.height() as i32;
        let rgba = img.as_raw();

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: (width * height * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(std::ptr::null_mut());
        let hbitmap = CreateDIBSection(
            hdc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        ReleaseDC(std::ptr::null_mut(), hdc);

        if hbitmap.is_null() || bits.is_null() {
            return std::ptr::null_mut();
        }

        let dst = bits as *mut u8;
        for i in 0..(width * height) as usize {
            let src = &rgba[i * 4..];
            *dst.add(i * 4) = src[2];
            *dst.add(i * 4 + 1) = src[1];
            *dst.add(i * 4 + 2) = src[0];
            *dst.add(i * 4 + 3) = src[3];
        }

        let hmask = CreateBitmap(width, height, 1, 1, std::ptr::null_mut());
        if hmask.is_null() {
            DeleteObject(hbitmap);
            return std::ptr::null_mut();
        }

        let mut icon_info = std::mem::zeroed::<ICONINFO>();
        icon_info.fIcon = 1;
        icon_info.hbmColor = hbitmap;
        icon_info.hbmMask = hmask;

        let hicon = CreateIconIndirect(&icon_info);

        DeleteObject(hbitmap);
        DeleteObject(hmask);

        hicon
    }
}

/// Load embedded icon PNG as tray_icon::Icon
fn load_tray_icon() -> anyhow::Result<tray_icon::Icon> {
    let rgba = image::load_from_memory(include_bytes!("../icons/icon.png"))
        .map_err(|e| anyhow::anyhow!("Failed to load tray icon: {e}"))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(tray_icon::Icon::from_rgba(rgba.into_raw(), width, height)?)
}

/// Create a hidden popup window for TrackPopupMenu ownership.
fn create_hidden_window() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let class_name: Vec<u16> = "BandHeartRateTray\0".encode_utf16().collect();
    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wnd_class); // may fail if already registered, which is fine
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            0 as windows_sys::Win32::Foundation::HWND,
            std::ptr::null_mut(),
            h_instance,
            std::ptr::null(),
        )
    }
}

/// Show the tray context menu at the current cursor position.
/// Uses TPM_RETURNCMD so it blocks and returns the selected item ID (0 if cancelled).
/// Follows Microsoft's recommended pattern (KB Q135788):
/// SetForegroundWindow → TrackPopupMenu → PostMessage(WM_NULL)
/// This ensures the menu auto-dismisses when clicking outside.
fn show_tray_context_menu(hwnd: windows_sys::Win32::Foundation::HWND, menu: &muda::Menu) -> u32 {
    use muda::ContextMenu;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, PostMessageW, SetForegroundWindow, TrackPopupMenu, TPM_BOTTOMALIGN,
        TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
    };
    unsafe {
        // Required before TrackPopupMenu per Microsoft docs
        SetForegroundWindow(hwnd);

        let mut point: windows_sys::Win32::Foundation::POINT = std::mem::zeroed();
        GetCursorPos(&mut point);
        let result = TrackPopupMenu(
            menu.hpopupmenu() as *mut _,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null_mut(),
        ) as u32;

        // Required after TrackPopupMenu per Microsoft docs
        // Ensures menu is dismissed when focus moves away
        PostMessageW(hwnd, WM_NULL, 0, 0);

        result
    }
}

/// Pump Windows messages so tray-icon's WndProc can receive events
fn pump_windows_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG,
    };
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, 1) } != 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Run the Slint GUI, consuming heart rate data from the watch channel.
pub fn run(
    rx: watch::Receiver<HeartRateReading>,
    config_rx: watch::Receiver<Config>,
    config_tx: watch::Sender<Config>,
) -> anyhow::Result<()> {
    let window = MainWindow::new()?;

    // Apply initial settings from config
    {
        let config = config_rx.borrow();
        window.set_input_max_hr(config.max_heart_rate.to_string().into());
        window.set_input_devices(config.allowed_devices.clone().into());
        window.set_input_port(config.server_port.to_string().into());
        window.set_setting_auto_start(config.auto_start);
        window.set_setting_minimize_tray(config.minimize_to_tray);
    }

    // Create a hidden window for popup menu ownership
    let menu_hwnd = create_hidden_window();

    // Create the popup menu
    let menu = muda::Menu::new();
    let quit_item = muda::MenuItem::with_id(
        MENU_QUIT,
        "退出",
        true,
        None::<muda::accelerator::Accelerator>,
    );
    let _ = menu.append(&quit_item);

    // Set window icon using PNG data converted to HICON via CreateIconIndirect
    // Use EnumWindows + process ID to find the Slint window handle reliably
    // Keep _icon_timer alive until the end of the function
    let _icon_timer = slint::Timer::default();
    let _retry_count = Rc::new(Cell::new(0u32));
    let _icon_set = Rc::new(Cell::new(false));
    {
        let retry_count2 = _retry_count.clone();
        let icon_set2 = _icon_set.clone();
        _icon_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(200),
            move || {
                if icon_set2.get() {
                    return; // Icon already set, skip
                }
                if retry_count2.get() >= 10 {
                    return; // Give up after 10 retries (2 seconds)
                }
                retry_count2.set(retry_count2.get() + 1);

                unsafe {
                    use windows_sys::Win32::Foundation::*;
                    use windows_sys::Win32::System::Threading::*;
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;

                    let current_pid = GetCurrentProcessId();

                    struct EnumData {
                        pid: u32,
                        hwnd: HWND,
                    }
                    let mut data = EnumData {
                        pid: current_pid,
                        hwnd: std::ptr::null_mut(),
                    };

                    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                        let data = &mut *(lparam as *mut EnumData);
                        let mut pid: u32 = 0;
                        GetWindowThreadProcessId(hwnd, &mut pid);
                        if pid == data.pid && IsWindowVisible(hwnd) != 0 {
                            let mut buf = [0u16; 256];
                            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 256);
                            if len > 0 {
                                data.hwnd = hwnd;
                                return 0;
                            }
                        }
                        1
                    }

                    EnumWindows(Some(enum_callback), &mut data as *mut EnumData as LPARAM);
                    let hwnd = data.hwnd;

                    if hwnd.is_null() {
                        return;
                    }

                    let icon_data = include_bytes!("../icons/icon.png");

                    let hicon_big = create_hicon_from_png(icon_data, 48);
                    if !hicon_big.is_null() {
                        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, hicon_big as _);
                    }

                    let hicon_sm = create_hicon_from_png(icon_data, 16);
                    if !hicon_sm.is_null() {
                        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, hicon_sm as _);
                        icon_set2.set(true); // Both icons set successfully
                    }
                }
            },
        );
    }

    // Set up system tray — NO .with_menu(), handle right-click manually
    let icon = load_tray_icon().ok();
    let _tray = icon.and_then(|icon| {
        tray_icon::TrayIconBuilder::new()
            .with_tooltip("Band Heart Rate Monitor")
            .with_icon(icon)
            .build()
            .ok()
    });

    // Intercept window close: hide and let window.run() return
    let w = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(win) = w.upgrade() {
            let _ = win.window().hide();
        }
        slint::CloseRequestResponse::HideWindow
    });

    // Stats state
    let stats_min = Rc::new(Cell::new(0u16));
    let stats_max = Rc::new(Cell::new(0u16));
    let stats_sum = Rc::new(Cell::new(0u64));
    let stats_count = Rc::new(Cell::new(0u32));

    // Reset button handler
    {
        let window_weak = window.as_weak();
        let sm = stats_min.clone();
        let sx = stats_max.clone();
        let ss = stats_sum.clone();
        let sc = stats_count.clone();
        window.on_reset_stats(move || {
            sm.set(0);
            sx.set(0);
            ss.set(0);
            sc.set(0);
            if let Some(w) = window_weak.upgrade() {
                w.set_stat_min("--".into());
                w.set_stat_max("--".into());
                w.set_stat_avg("--".into());
                w.set_bpm_text("--".into());
                w.set_zone_label("--".into());
                w.set_zone_color(parse_color("#4A4F58"));
                w.set_zone_bg_color(zone_bg_color("#4A4F58"));
            }
        });
    }

    // Settings callbacks
    {
        let window_weak = window.as_weak();
        window.on_open_settings(move || {
            if let Some(w) = window_weak.upgrade() {
                w.set_settings_visible(true);
            }
        });
    }
    {
        let window_weak = window.as_weak();
        window.on_close_settings(move || {
            if let Some(w) = window_weak.upgrade() {
                w.set_settings_visible(false);
            }
        });
    }
    // Copy address callback
    {
        let window_weak = window.as_weak();
        let rx_clone = rx.clone();
        window.on_copy_address(move || {
            let data = rx_clone.borrow().clone();
            if data.device_address.is_some() {
                let port = window_weak
                    .upgrade()
                    .map(|w| w.get_input_port().to_string())
                    .unwrap_or_else(|| "3030".into());
                let text = format!("http://127.0.0.1:{port}");
                #[cfg(target_os = "windows")]
                {
                    use clipboard_win::{formats::Unicode, set_clipboard};
                    let _ = set_clipboard(Unicode, &text);
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = text;
                }
            }
        });
    }
    {
        let window_weak = window.as_weak();
        let config_tx_clone = config_tx.clone();
        window.on_save_settings(move |max_hr, devices, port, auto_start, min_tray| {
            let new_config = Config {
                max_heart_rate: max_hr.to_string().trim().parse().unwrap_or(190),
                allowed_devices: devices.to_string(),
                server_port: port.to_string().trim().parse().unwrap_or(3030),
                auto_start,
                minimize_to_tray: min_tray,
            };
            let _ = new_config.save();
            let _ = config_tx_clone.send(new_config);
            if let Some(w) = window_weak.upgrade() {
                w.set_settings_visible(false);
            }
        });
    }

    // Timer for heart rate data
    let window_weak = window.as_weak();
    let rx = rx.clone();

    // Cache previous values to avoid unnecessary UI updates (causes flickering)
    let prev_hr = Rc::new(Cell::new(0u16));
    let prev_status = Rc::new(Cell::new(0u8)); // 0: disconnected, 1: scanning, 2: connected, 3: error
    let prev_addr = Rc::new(RefCell::new(String::new()));

    // Flag to signal right-click on tray while window was visible
    let right_click_pending = Rc::new(Cell::new(false));

    let _timer = slint::Timer::default();
    let prev_hr_clone = prev_hr.clone();
    let prev_status_clone = prev_status.clone();
    let prev_addr_clone = prev_addr.clone();
    let config_rx_for_timer = config_rx.clone();

    // Share menu with tray timer via Rc
    let menu = Rc::new(menu);
    let right_click_pending2 = right_click_pending.clone();

    _timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            // Always borrow the latest value from the watch channel
            let data = rx.borrow().clone();
            let max_hr = config_rx_for_timer.borrow().max_heart_rate;

            let Some(w) = window_weak.upgrade() else {
                return;
            };

            // Status - only update if changed
            let new_status = if data.error.is_some() {
                3
            } else if data.connected {
                2
            } else if data.scanning {
                1
            } else {
                0
            };

            if prev_status_clone.get() != new_status {
                prev_status_clone.set(new_status);
                if let Some(ref err) = data.error {
                    w.set_status_text(format!("错误: {err}").into());
                    w.set_status_color(parse_color("#EF4444"));
                } else if data.connected {
                    w.set_status_text("已连接".into());
                    w.set_status_color(parse_color("#4ADE80"));
                } else if data.scanning {
                    w.set_status_text("扫描中...".into());
                    w.set_status_color(parse_color("#FBBF24"));
                } else {
                    w.set_status_text("断开连接".into());
                    w.set_status_color(parse_color("#EF4444"));
                }
            }

            // Broadcast address - only update if changed
            let new_addr = data.device_address.clone().unwrap_or_default();
            if *prev_addr_clone.borrow() != new_addr {
                *prev_addr_clone.borrow_mut() = new_addr.clone();
                let display = match &data.device_name {
                    Some(name) if !new_addr.is_empty() => name.clone(),
                    _ => "".to_string(),
                };
                w.set_broadcast_address(display.into());
            }

            // Heart rate - only update if changed
            let new_hr = if data.connected && data.heart_rate > 0 {
                data.heart_rate
            } else {
                0
            };

            if prev_hr_clone.get() != new_hr {
                prev_hr_clone.set(new_hr);

                if new_hr > 0 {
                    w.set_bpm_text(new_hr.to_string().into());

                    // Zone
                    let (zone_label, zone_color) = get_zone(new_hr, max_hr);
                    w.set_zone_label(zone_label.into());
                    w.set_zone_color(parse_color(zone_color));
                    w.set_zone_bg_color(zone_bg_color(zone_color));

                    // Stats
                    let hr = new_hr;
                    if stats_count.get() == 0 {
                        stats_min.set(hr);
                        stats_max.set(hr);
                        stats_sum.set(hr as u64);
                        stats_count.set(1);
                    } else {
                        if hr < stats_min.get() {
                            stats_min.set(hr);
                        }
                        if hr > stats_max.get() {
                            stats_max.set(hr);
                        }
                        stats_sum.set(stats_sum.get() + hr as u64);
                        stats_count.set(stats_count.get() + 1);
                    }

                    w.set_stat_min(stats_min.get().to_string().into());
                    w.set_stat_max(stats_max.get().to_string().into());
                    let avg = if stats_count.get() > 0 {
                        (stats_sum.get() / stats_count.get() as u64) as u16
                    } else {
                        0
                    };
                    w.set_stat_avg(avg.to_string().into());
                } else if !data.connected && !data.scanning {
                    w.set_bpm_text("--".into());
                    w.set_zone_label("--".into());
                    w.set_zone_color(parse_color("#4A4F58"));
                    w.set_zone_bg_color(zone_bg_color("#4A4F58"));
                }
            }

            // Non-blocking tray event drain while window is visible
            while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                match event {
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        // Bring to foreground if already visible
                        let _ = w.window().show();
                    }
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Right,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        // Signal right-click pending and hide window
                        // Main loop will show the context menu after window.run() returns
                        right_click_pending2.set(true);
                        let _ = w.window().hide();
                    }
                    _ => {}
                }
            }
        },
    );

    // Main loop
    let mut should_quit = false;

    while !should_quit {
        window.run()?;

        // window.run() returned — window was hidden via HideWindow

        // Check if a right-click was requested while window was visible
        if right_click_pending.get() {
            right_click_pending.set(false);
            // Drain the event queue first
            pump_windows_messages();
            while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
            while muda::MenuEvent::receiver().try_recv().is_ok() {}
            // Show context menu
            let selected = show_tray_context_menu(menu_hwnd, menu.as_ref());
            while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
            if selected > 0 {
                break;
            }
            // Re-show window after menu dismissed
            let _ = window.window().show();
            continue;
        }

        // Wait briefly and flush any residual events
        std::thread::sleep(std::time::Duration::from_millis(300));
        pump_windows_messages();
        while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
        while muda::MenuEvent::receiver().try_recv().is_ok() {}

        // Poll tray events
        loop {
            pump_windows_messages();

            let mut want_show = false;
            while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                match event {
                    // Left single-click → show window
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        want_show = true;
                    }
                    // Right single-click → show context menu manually
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Right,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        let selected = show_tray_context_menu(menu_hwnd, menu.as_ref());
                        // TrackPopupMenu with TPM_RETURNCMD blocks until menu is closed.
                        // It returns 0 if user clicks outside (loses focus) or presses Esc.
                        // It returns the menu item ID if user selects one.
                        // Drain any residual tray events generated during menu display.
                        while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
                        if selected > 0 {
                            should_quit = true;
                        }
                    }
                    _ => {}
                }
            }

            // Also check menu events from Slint event loop phase
            while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                if event.id().0.as_str() == MENU_QUIT {
                    should_quit = true;
                    break;
                }
            }

            if should_quit || want_show {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if !should_quit {
            let _ = window.window().show();
        }
    }

    Ok(())
}
