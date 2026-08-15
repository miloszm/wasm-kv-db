use crate::app::AppState;
use crate::app::models::ErrorResponse;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use rmp_serde::Deserializer;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericRequest {
    pub tenant_id: String,
    pub reducer_name: String,
    pub reducer_args: Vec<u8>, // MessagePack serialized args
    pub caller_id: String,
    pub timestamp: u64,
}

fn from_msgpack<T: DeserializeOwned>(data: &[u8]) -> T {
    let mut deserializer = Deserializer::new(Cursor::new(data));
    T::deserialize(&mut deserializer).unwrap()
}

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

/// POST /kvexec
/// Executes tenant's Wasm
pub(crate) async fn execute(State(state): State<AppState>, body: Bytes) -> impl IntoResponse {
    let request: GenericRequest = from_msgpack(body.as_ref());
    println!("loading raffle.wasm with caller id: {}",request.caller_id.clone());
    match state.load_wasm_guest(
        "t01",
        "wasm-guests/raffle/target/wasm32-unknown-unknown/debug/raffle.wasm",
        state.storage.clone(),
        request.caller_id.clone(),
    ) {
        Ok(_) => (),
        Err(e) => {
            return (StatusCode::NOT_FOUND, e.to_string()).into_response();
        }
    }
    if let Some(mut guest) = state.wasm_guests.get_mut(&request.tenant_id) {
        let name_bytes = request.reducer_name.as_bytes();
        match guest.execute(&name_bytes, &request.reducer_args) {
            Ok(result) => {
                println!("returning {:?}", result);
                (StatusCode::OK, result).into_response()
            },
            Err(e) => {
                eprintln!("Wasm execution failed: {}", e);
                let err = ErrorResponse {
                    error: format!(
                        "Guest execution failed for tenant: {} reducer name: {} user: {}",
                        request.tenant_id, request.reducer_name, request.caller_id
                    ),
                };
                (StatusCode::OK, err.error).into_response()
            }
        }
    } else {
        let err = ErrorResponse {
            error: format!("Guest not found for {}", request.reducer_name),
        };
        (StatusCode::NOT_FOUND, err.error).into_response()
    }
}

/// GET /kv
pub(crate) async fn list_keys(State(state): State<AppState>) -> impl IntoResponse {
    let keys: Vec<String> = state.storage.list_keys();
    (StatusCode::OK, Json(keys)).into_response()
}
