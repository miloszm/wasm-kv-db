use crate::app::AppState;
use crate::app::models::ErrorResponse;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

/// GET /kv/{tenant}/{key}
pub(crate) async fn get_value(
    State(state): State<AppState>,
    Path((tenant, key)): Path<(String, String)>,
) -> impl IntoResponse {
    let full_key = format!("{}:{}", tenant, key);
    let value = match state.storage.get(&full_key) {
        Ok(value) => value.clone(),
        Err(_) => {
            let err = ErrorResponse {
                error: format!("Key '{}/{}' not found", tenant, key),
            };
            return (StatusCode::NOT_FOUND, Json(err)).into_response();
        }
    };

    (StatusCode::OK, Json(value)).into_response()
}

/// PUT /kv/{tenant}/{name}
/// Executes tenant's Wasm
pub(crate) async fn execute(
    State(state): State<AppState>,
    Path((tenant, name)): Path<(String, String)>,
    body: Bytes,
) -> impl IntoResponse {
    if let Some(mut guest) = state.wasm_guests.get_mut(&tenant) {
        match guest.execute(&body) {
            Ok(result) => (StatusCode::OK, Json(result)).into_response(),
            Err(e) => {
                eprintln!("Wasm execution failed: {}", e);
                let err = ErrorResponse {
                    error: format!("Guest execution failed for {tenant}/{name}"),
                };
                (StatusCode::OK, Json(err)).into_response()
            }
        }
    } else {
        let err = ErrorResponse {
            error: format!("Guest not found for {name}"),
        };
        (StatusCode::NOT_FOUND, Json(err)).into_response()
    }
}

/// GET /kv
pub(crate) async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state.storage.list_keys();
    (StatusCode::OK, Json(keys)).into_response()
}
