pub mod handlers;
pub mod models;

use crate::error::AppError;
use crate::wasm::WasmGuest;
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DashMap<String, serde_json::Value>>,
    pub wasm_guests: Arc<DashMap<String, WasmGuest>>, // tenant_id -> WasmGuest
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            wasm_guests: Arc::new(DashMap::new()),
        }
    }

    pub fn load_wasm_guest(
        &self,
        tenant_id: &str,
        wasm_path: impl AsRef<Path>,
    ) -> Result<(), AppError> {
        let wasm_bytes = std::fs::read(wasm_path)?;
        let guest = WasmGuest::new(&wasm_bytes)?;
        self.wasm_guests.insert(tenant_id.to_string(), guest);
        Ok(())
    }
}
