use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{extract::State, Json, Router};

use crate::config::Config;
use crate::web_ui;
use tokio::sync::watch;

use crate::types::{AppState, HeartRateReading};

pub async fn run_server(
    rx: watch::Receiver<HeartRateReading>,
    config_tx: watch::Sender<Config>,
    config_rx: watch::Receiver<Config>,
) -> anyhow::Result<()> {
    let port = config_rx.borrow().server_port;
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .route("/heart-rate-stream", get(heart_rate_sse))
        .route("/health", get(health))
        .route("/settings", get(get_settings).put(update_settings))
        .with_state(AppState {
            rx,
            config_rx,
            config_tx,
        });

    let addr = format!("127.0.0.1:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => {
            tracing::info!("Web UI 运行在 http://{addr}/");
            l
        }
        Err(e) => {
            tracing::warn!("端口 {port} 绑定失败: {e}，尝试随机端口...");
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = l.local_addr()?.port();
            tracing::info!("Web UI 运行在 http://127.0.0.1:{port}/");
            l
        }
    };

    axum::serve(listener, app).await?;
    Ok(())
}

async fn heart_rate(State(state): State<AppState>) -> Json<HeartRateReading> {
    Json(state.rx.borrow().clone())
}

/// SSE 推送心率数据流
async fn heart_rate_sse(
    State(state): State<AppState>,
) -> Sse<impl futures_lite::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.rx.clone();
    let stream = futures_lite::stream::unfold(rx, |mut rx| async move {
        if rx.changed().await.is_ok() {
            let reading = rx.borrow().clone();
            let data = serde_json::to_string(&reading).unwrap_or_default();
            Some((Ok(Event::default().data(data)), rx))
        } else {
            None
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn index() -> axum::response::Html<&'static str> {
    axum::response::Html(web_ui::HTML)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn get_settings(State(state): State<AppState>) -> Json<Config> {
    Json(state.config_rx.borrow().clone())
}

async fn update_settings(
    State(state): State<AppState>,
    Json(new_config): Json<Config>,
) -> Json<Config> {
    let old_port = state.config_rx.borrow().server_port;
    new_config.save().ok();
    state.config_tx.send(new_config.clone()).ok();
    if new_config.server_port != old_port {
        tracing::warn!(
            "HTTP 端口已更改为 {}，需要重启才能生效",
            new_config.server_port
        );
    }
    Json(new_config)
}
