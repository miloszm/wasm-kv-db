use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use tracing::info;
use crate::app::AppState;
use crate::app::models::{ErrorResponse, KvEntry, TransformParams};

/// GET /kv/{key}
pub(crate) async fn get_value(
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
pub(crate) async fn put_value(
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
pub(crate) async fn delete_value(State(state): State<AppState>, Path(key): Path<String>) -> impl IntoResponse {
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
pub(crate) async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state
        .store
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    (StatusCode::OK, Json(keys)).into_response()
}
