pub mod app;
pub mod storage;

use wasm_kv_db::{AppError, error, wasm};
use crate::app::AppState;
use crate::app::handlers::{delete_value, get_value, list_keys, put_value};
use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use tokio::signal;
use tracing::{info, warn};

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt().init();

    info!("Starting Wasm KV Database...");

    let state = AppState::new();

    // todo: test code to be moved away
    // guests should be loaded by tenants via REST API
    // possibly default guests should be provided for new tenants
    state.load_wasm_guest(
        "t01",
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/kv", get(list_keys))
        .route("/kv/:tenant/:key", get(get_value))
        .route("/kv/:tenant/:key", put(put_value))
        .route("/kv/:tenant/:key", delete(delete_value))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Listening on http://localhost:8080");

    tokio::select! {
        result = axum::serve(listener, app) => {
            if let Err(e) = result {
                warn!("Server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            info!("Shutting down gracefully...");
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
