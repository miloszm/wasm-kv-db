pub mod handlers;
pub mod models;

use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use crate::error::AppError;
use crate::wasm::WasmGuest;

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

