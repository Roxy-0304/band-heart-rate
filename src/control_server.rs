use axum::response::Html;
use axum::routing::{get, post};
use axum::{extract::State, Json, Router};
use serde::Deserialize;

use crate::control_ui::CONTROL_HTML;
use crate::types::{BleCommand, ControlState, DiscoveredDevice};

pub async fn run_control_server(state: ControlState, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .route("/devices", get(devices))
        .route("/devices/select", post(select_device))
        .route("/devices/disconnect", post(post_disconnect))
        .route("/settings", get(get_settings).put(update_settings))
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

async fn get_settings(State(state): State<ControlState>) -> Json<crate::config::Config> {
    Json(state.config_rx.borrow().clone())
}

async fn update_settings(
    State(state): State<ControlState>,
    Json(new_config): Json<crate::config::Config>,
) -> Json<crate::config::Config> {
    new_config.save().ok();
    state.config_tx.send(new_config.clone()).ok();
    Json(new_config)
}
