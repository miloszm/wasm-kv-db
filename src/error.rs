use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Wasm error: {0}")]
    Wasm(#[from] wasmtime::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Wasm guest error: {0}")]
    WasmGuest(String),

    #[allow(unused)]
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[allow(unused)]
    #[error("Wasm module not loaded")]
    WasmNotLoaded,
}
