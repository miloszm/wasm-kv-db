use crate::app::AppState;
use crate::app::models::{ErrorResponse, KvEntry, TransformParams};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::info;

/// GET /kv/{tenant}/{key}
pub(crate) async fn get_value(
    State(state): State<AppState>,
    Path((tenant, key)): Path<(String, String)>,
    Query(params): Query<TransformParams>,
) -> impl IntoResponse {
    let full_key = format!("{}:{}", tenant, key);
    let value = match state.storage.get_raw(&full_key) {
        Ok(value) => value.clone(),
        Err(_) => {
            let err = ErrorResponse {
                error: format!("Key '{}/{}' not found", tenant, key),
            };
            return (StatusCode::NOT_FOUND, Json(err)).into_response();
        }
    };

    if params.transform {
        if let Some(mut guest) = state.wasm_guests.get_mut(&tenant) {
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

/// PUT /kv/{tenant}/{key}
pub(crate) async fn put_value(
    State(state): State<AppState>,
    Path((tenant, key)): Path<(String, String)>,
    Json(value): Json<serde_json::Value>,
) -> impl IntoResponse {
    let full_key = format!("{}:{}", tenant, key);
    match state.storage.put(full_key.as_str(), value) {
        Ok(value) => {
            info!("PUT /kv/{}/{} -> stored", tenant, key);
            let response = KvEntry { key, value };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(_) => {
            let err = ErrorResponse {
                error: format!("Problem entering key '{}/{}'", tenant, key),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
    }
}

/// DELETE /kv/{tenant}/{key}
pub(crate) async fn delete_value(
    State(state): State<AppState>,
    Path((tenant, key)): Path<(String, String)>,
) -> impl IntoResponse {
    let full_key = format!("{}:{}", tenant, key);
    match state.storage.delete(&full_key) {
        Ok(value) => {
            info!("DELETE /kv/{}/{} -> removed", tenant, key);
            let response = KvEntry { key, value };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let err = ErrorResponse {
                error: format!("Key '{}/{}' not found", tenant, key),
            };
            (StatusCode::NOT_FOUND, Json(err)).into_response()
        }
    }
}

/// GET /kv
pub(crate) async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state.storage.list_keys();
    (StatusCode::OK, Json(keys)).into_response()
}
