pub mod handlers;
pub mod models;

use crate::error::AppError;
use crate::wasm::WasmGuest;
use dashmap::DashMap;
use std::path::Path;
use std::sync::Arc;
use wasm_kv_db::Storage;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub wasm_guests: Arc<DashMap<String, WasmGuest>>, // tenant_id -> WasmGuest
}

impl AppState {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, AppError> {
        Ok(Self {
            storage: Storage::new_with_persistence(path)?,
            wasm_guests: Arc::new(DashMap::new()),
        })
    }

    pub fn load_wasm_guest(
        &self,
        tenant_id: &str,
        wasm_path: impl AsRef<Path>,
        storage: Storage,
        user_id: impl AsRef<str>,
    ) -> Result<(), AppError> {
        let wasm_bytes = std::fs::read(wasm_path)?;
        let guest = WasmGuest::new(&wasm_bytes, storage, user_id)?;
        self.wasm_guests.insert(tenant_id.to_string(), guest);
        Ok(())
    }
}
