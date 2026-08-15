use crate::error::AppError;
use dashmap::DashMap;
use rocksdb::{DB, IteratorMode, Options};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tracing::{error, info};

/// In-memory key-value store with optional Wasm transformation
#[derive(Clone)]
pub struct Storage {
    cache: Arc<DashMap<String, Vec<u8>>>,
    db: Arc<DB>,
    write_tx: mpsc::UnboundedSender<(String, Vec<u8>)>,
    #[allow(unused)]
    temp_dir: Option<Arc<TempDir>>,
}

impl Storage {
    pub fn new_with_persistence<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_write_buffer_size(64 * 1024 * 1024);
        opts.set_max_write_buffer_number(3);
        opts.set_min_write_buffer_number_to_merge(2);

        let db = DB::open(&opts, path)?;
        let db = Arc::new(db);
        let cache = Arc::new(DashMap::new());

        info!("Loading existing data from DB");
        let iter = db.iterator(IteratorMode::Start);
        let mut count = 0;
        for item in iter {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())
                .map_err(|e| AppError::Serialization(format!("Invalid encoding: {e}")))?;
            cache.insert(key_str, value.to_vec());
            count += 1;
        }
        info!("Loaded {count} keys from DB");

        // Channel for background writes
        let (tx, mut rx) = mpsc::unbounded_channel();
        let db_clone = db.clone();

        // Background writer task
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(100);
            let mut total_writes = 0;

            while let Some((key, value)) = rx.recv().await {
                batch.push((key, value));
                if batch.len() >= 100 {
                    if let Err(e) = Self::flush_batch(&db_clone, &batch) {
                        error!("Failed to flush batch to DB: {e}")
                    }
                    batch.clear();
                    total_writes += batch.len();
                }
            }
            // On shutdown
            if !batch.is_empty() {
                if let Err(e) = Self::flush_batch(&db_clone, &batch) {
                    error!("Failed to flush batch to DB: {e}")
                }
            }
            info!("DB writer task finished. Total writes: {total_writes}");
        });

        Ok(Self {
            cache,
            db,
            write_tx: tx,
            temp_dir: None,
        })
    }

    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let path = temp_dir.path().join("rocksdb");
        std::fs::create_dir_all(&path).expect("Failed to create DB directory");

        let mut opts = Options::default();
        opts.create_if_missing(true);
        let temp_dir = tempfile::tempdir().unwrap();
        let db = DB::open(&opts, &path).unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();

        Self {
            cache: Arc::new(DashMap::new()),
            db: Arc::new(db),
            write_tx: tx,
            temp_dir: Some(Arc::new(temp_dir)),
        }
    }

    /// Insert or update a value
    pub fn put(&self, key: &str, value: Vec<u8>) -> Result<Vec<u8>, AppError> {
        self.cache.insert(key.to_string(), value.clone());

        if let Err(e) = self.write_tx.send((key.to_string(), value.clone())) {
            error!("Failed to write to queue for key '{key}': {e}")
        }

        Ok(value)
    }

    /// Retrieve a value by key
    pub fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        self.cache
            .get(key)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// Retrieve an int value by key
    pub fn get_int(&self, key: &str) -> Result<i64, AppError> {
        let bytes = self.get(key)?;
        let len = bytes.len();
        bytes.try_into().map(i64::from_le_bytes).map_err(|_| {
            AppError::Serialization(format!(
                "Value for key '{}' is not a valid i64 (len={})",
                key, len
            ))
        })
    }

    /// Retrieve length of a value by key
    pub fn get_len(&self, key: &str) -> Result<usize, AppError> {
        self.cache
            .get(key)
            .map(|entry| entry.value().len())
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let value = self
            .cache
            .remove(key)
            .map(|(_, value)| value)
            .ok_or_else(|| AppError::KeyNotFound(key.to_string()))?;

        if let Err(e) = self.write_tx.send((key.to_string(), Vec::new())) {
            error!("Failed to write to queue for key '{key}': {e}")
        }

        Ok(value)
    }

    /// Append to list
    pub fn append_to_list(&self, key: &str, v: Vec<u8>) -> Result<(), AppError> {
        self.cache
            .entry(key.to_string())
            .and_modify(|existing| existing.extend(v.clone()))
            .or_insert(v);

        if let Some(entry) = self.cache.get(key) {
            let value = entry.value().clone();
            if let Err(e) = self.write_tx.send((key.to_string(), value)) {
                error!("Failed to queue append for key '{}': {}", key, e);
            }
        }

        Ok(())
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.cache.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Check if a key exists
    pub fn exists(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.cache.clear();
        // inefficient
        for key in self.list_keys() {
            let _ = self.delete(&key);
        }
    }

    /// Sync with DB
    pub fn sync(&self) -> Result<(), AppError> {
        // inefficient
        for entry in self.cache.iter() {
            let key = entry.key();
            let value = entry.value();
            self.db.put(key.as_bytes(), value)?;
        }
        Ok(())
    }

    /// Helper to flush a batch to DB
    fn flush_batch(db: &DB, batch: &[(String, Vec<u8>)]) -> Result<(), rocksdb::Error> {
        let mut write_batch = rocksdb::WriteBatch::default();

        for (key, value) in batch {
            if value.is_empty() {
                write_batch.delete(key.as_bytes());
            } else {
                write_batch.put(key.as_bytes(), value);
            }
        }
        db.write(write_batch)?;
        Ok(())
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_db_path() -> String {
    let home = std::env::var("HOME").unwrap_or(".".to_string());
    format!("{home}/.wasm-kv-db/db")
}
