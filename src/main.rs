mod wasm;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

// ---------- DATA MODELS ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvEntry {
    pub key: String,
    pub value: serde_json::Value, // Flexible JSON value
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ---------- APPLICATION STATE ----------

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DashMap<String, serde_json::Value>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }
}

// ---------- HANDLERS ----------

/// GET /kv/{key}
/// Retrieve a value by key
async fn get_value(State(state): State<AppState>, Path(key): Path<String>) -> impl IntoResponse {
    match state.store.get(&key) {
        Some(entry) => {
            let value = entry.value().clone();
            (StatusCode::OK, Json(value)).into_response()
        }
        None => {
            let err = ErrorResponse {
                error: format!("Key '{}' not found", key),
            };
            (StatusCode::NOT_FOUND, Json(err)).into_response()
        }
    }
}

/// PUT /kv/{key}
/// Insert or update a value
async fn put_value(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(value): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Insert or update
    state.store.insert(key.clone(), value.clone());

    info!("PUT /kv/{} -> stored", key);

    let response = KvEntry { key, value };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// DELETE /kv/{key}
/// Remove a key
async fn delete_value(State(state): State<AppState>, Path(key): Path<String>) -> impl IntoResponse {
    match state.store.remove(&key) {
        Some((_, value)) => {
            info!("DELETE /kv/{} -> removed", key);
            let response = KvEntry { key, value };
            (StatusCode::OK, Json(response)).into_response()
        }
        None => {
            let err = ErrorResponse {
                error: format!("Key '{}' not found", key),
            };
            (StatusCode::NOT_FOUND, Json(err)).into_response()
        }
    }
}

/// GET /kv
/// List all keys (optional utility)
async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state
        .store
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    (StatusCode::OK, Json(keys)).into_response()
}

// ---------- HEALTH CHECK ----------

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// ---------- MAIN ----------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        // .with_env_filter("info")
        .init();

    info!("Starting Wasm KV Database...");

    let state = AppState::new();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/kv", get(list_keys))
        .route("/kv/:key", get(get_value))
        .route("/kv/:key", put(put_value))
        .route("/kv/:key", delete(delete_value))
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

/// Signal handler for graceful shutdown
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::WasmGuest;

    #[test]
    pub fn test_wasm_guest() -> Result<(), anyhow::Error> {
        let wasm_bytes = std::fs::read(
            "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
        )?;

        let mut guest = WasmGuest::new(&wasm_bytes)?;

        let input = b"hello";

        let output = guest.transform(input)?;

        println!("Input:  {}", String::from_utf8_lossy(input));
        println!("Output: {}", String::from_utf8_lossy(&output));

        Ok(())
    }
}
