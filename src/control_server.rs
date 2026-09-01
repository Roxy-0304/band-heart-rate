use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router, extract::State};
use serde::Deserialize;

use crate::control_ui::CONTROL_HTML;
use crate::types::{BleCommand, ControlState, DiscoveredDevice};
use crate::version_check;

pub async fn run_control_server(state: ControlState, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .route("/devices", get(devices))
        .route("/devices/select", post(select_device))
        .route("/devices/disconnect", post(post_disconnect))
        .route("/devices/rescan", post(post_rescan))
        .route("/settings", get(get_settings).put(update_settings))
        .route("/version", get(version_info))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => {
            tracing::info!("控制面板运行在 http://{addr}/");
            l
        }
        Err(e) => {
            tracing::warn!("控制面板端口 {port} 绑定失败: {e}，尝试随机端口...");
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = l.local_addr()?.port();
            tracing::info!("控制面板运行在 http://127.0.0.1:{port}/");
            l
        }
    };

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(CONTROL_HTML)
}

async fn heart_rate(State(state): State<ControlState>) -> Json<crate::types::HeartRateReading> {
    Json(state.rx.borrow().clone())
}

async fn devices(State(state): State<ControlState>) -> Json<Vec<DiscoveredDevice>> {
    let list = state
        .discovered
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    Json(list)
}

#[derive(Deserialize)]
struct SelectReq {
    device_id: String,
}

async fn select_device(
    State(state): State<ControlState>,
    Json(req): Json<SelectReq>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    state
        .ble_cmd_tx
        .send(BleCommand::SelectDevice(req.device_id))
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "BLE service unavailable".to_string(),
            )
        })?;
    Ok(Json("ok"))
}

async fn post_disconnect(
    State(state): State<ControlState>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    state
        .ble_cmd_tx
        .send(BleCommand::Disconnect)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "BLE service unavailable".to_string(),
            )
        })?;
    Ok(Json("ok"))
}

async fn post_rescan(
    State(state): State<ControlState>,
) -> Result<Json<&'static str>, (axum::http::StatusCode, String)> {
    state
        .ble_cmd_tx
        .send(BleCommand::Rescan)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "BLE service unavailable".to_string(),
            )
        })?;
    Ok(Json("ok"))
}

async fn get_settings(State(state): State<ControlState>) -> Json<crate::config::Config> {
    Json(state.config_rx.borrow().clone())
}

async fn update_settings(
    State(state): State<ControlState>,
    Json(new_config): Json<crate::config::Config>,
) -> Json<crate::config::Config> {
    let config = crate::config::apply_config(&state.config_tx, new_config, None).await;
    Json(config)
}

#[derive(serde::Serialize)]
struct VersionInfo {
    current_version: String,
    latest_version: Option<String>,
    has_update: bool,
    release_url: String,
    error: Option<String>,
}

async fn version_info() -> Json<VersionInfo> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let result = version_check::check_latest_version().await;

    match result.latest_version {
        Some(latest) => {
            let has_update = version_check::is_newer_version(&latest, &current_version);
            Json(VersionInfo {
                current_version,
                latest_version: Some(latest),
                has_update,
                release_url: "https://github.com/Roxy-0304/band-heart-rate/releases"
                    .to_string(),
                error: None,
            })
        }
        None => Json(VersionInfo {
            current_version,
            latest_version: None,
            has_update: false,
            release_url: String::new(),
            error: result.error,
        }),
    }
}
