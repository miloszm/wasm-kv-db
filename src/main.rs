mod error;
mod wasm;

use crate::error::AppError;
use crate::wasm::WasmGuest;
use axum::extract::Query;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
// ------- data models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvEntry {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TransformParams {
    #[serde(default)]
    pub transform: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// -------- state

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DashMap<String, serde_json::Value>>,
    pub wasm_guest: Arc<Mutex<Option<WasmGuest>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            wasm_guest: Arc::new(Mutex::new(None)),
        }
    }

    pub fn ensure_wasm_loaded(&self, wasm_path: &str) -> Result<(), AppError> {
        let mut guard = self.wasm_guest.lock();
        if guard.is_none() {
            let wasm_bytes = std::fs::read(wasm_path)?;
            let guest = WasmGuest::new(&wasm_bytes)?;
            *guard = Some(guest);
        }
        Ok(())
    }
}

// --------- handlers

/// GET /kv/{key}
async fn get_value(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(params): Query<TransformParams>,
) -> impl IntoResponse {
    let value = match state.store.get(&key) {
        Some(entry) => entry.value().clone(),
        None => {
            let err = ErrorResponse {
                error: format!("Key '{}' not found", key),
            };
            return (StatusCode::NOT_FOUND, Json(err)).into_response();
        }
    };

    if params.transform {
        let mut guest_guard = state.wasm_guest.lock();
        if let Some(guest) = guest_guard.as_mut() {
            // let v = serde_json::from_str(r#"{"key": "value"}"#).unwrap();// temp, remove me
            match guest.transform_json(&value) {
                Ok(transformed) => return (StatusCode::OK, Json(transformed)).into_response(),
                Err(e) => {
                    eprintln!("Wasm transform failed: {}", e);
                    return (StatusCode::OK, Json(value)).into_response();
                }
            }
        }
    }

    (StatusCode::OK, Json(value)).into_response()
}

/// PUT /kv/{key}
async fn put_value(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(value): Json<serde_json::Value>,
) -> impl IntoResponse {
    state.store.insert(key.clone(), value.clone());
    info!("PUT /kv/{} -> stored", key);
    let response = KvEntry { key, value };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// DELETE /kv/{key}
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
async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state
        .store
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    (StatusCode::OK, Json(keys)).into_response()
}

// -------- health check

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt().init();

    info!("Starting Wasm KV Database...");

    let state = AppState::new();

    state.ensure_wasm_loaded(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;

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
    use crate::error::AppError;
    use crate::wasm::WasmGuest;

    #[test]
    pub fn test_wasm_guest() -> Result<(), AppError> {
        let wasm_bytes = std::fs::read(
            "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
        )?;

        let mut guest = WasmGuest::new(&wasm_bytes)?;

        // let input = serde_json::from_str(r#"{"key": "value"}"#)?;
        let input = serde_json::from_str(
            r#"{"department": "Engineering", "name": "Alice", "personal_email": "alice@gmail.com", "salary": 95000}"#,
        )?;

        let output = guest.transform_json(&input)?;

        println!("Input:  {}", input.to_string());
        println!("Output: {}", output.to_string());

        Ok(())
    }
}
