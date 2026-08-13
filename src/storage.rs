pub mod tenant_key;

use crate::error::AppError;
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory key-value store with optional Wasm transformation
#[derive(Clone)]
pub struct Storage {
    store: Arc<DashMap<String, Vec<u8>>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update a value
    pub fn put(&self, key: &str, value: Vec<u8>) -> Result<Vec<u8>, AppError> {
        self.store.insert(key.to_string(), value.clone());
        Ok(value)
    }

    /// Retrieve a value by key
    pub fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        self.store
            .get(key)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<Vec<u8>, AppError> {
        self.store
            .remove(key)
            .map(|(_, value)| value)
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// Append to list
    pub fn append_to_list(&self, key: &str, v: Vec<u8>) -> Result<(), AppError> {
        self.store
            .entry(key.to_string())
            .and_modify(|existing| existing.extend(v.clone()))
            .or_insert(v);
        Ok(())
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.store.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Check if a key exists
    pub fn exists(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.store.clear();
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}
