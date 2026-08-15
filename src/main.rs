pub mod app;
pub mod storage;

use crate::app::AppState;
use crate::app::handlers::{execute, get_value, list_keys};
use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use tokio::signal;
use tracing::{info, warn};
use wasm_kv_db::{AppError, error, wasm};

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt().init();

    info!("Starting Wasm KV Database...");

    let db_path = std::env::var("WASM_KV_DB_PATH").unwrap_or(storage::default_db_path());
    let state = AppState::new(db_path)?;

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/kv", get(list_keys))
        .route("/kv/:tenant/:key", get(get_value))
        .route("/kvexec", post(execute))
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
