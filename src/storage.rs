pub mod tenant_key;

use crate::error::AppError;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// In-memory key-value store with optional Wasm transformation
#[derive(Clone)]
pub struct Storage {
    store: Arc<DashMap<String, Value>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update a value
    pub fn put(&self, key: &str, value: Value) -> Result<Value, AppError> {
        self.store.insert(key.to_string(), value.clone());
        Ok(value)
    }

    /// Retrieve a value by key (without transformation)
    pub fn get_raw(&self, key: &str) -> Result<Value, AppError> {
        self.store
            .get(key)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<Value, AppError> {
        self.store
            .remove(key)
            .map(|(_, value)| value)
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.store.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Check if a key exists
    pub fn exists(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}
