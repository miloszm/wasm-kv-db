use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Wasm error: {0}")]
    Wasm(#[from] wasmtime::Error),

    #[error("Wasm guest error: {0}")]
    WasmGuest(String),

    #[allow(unused)]
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Buffer too short: needed {needed}, size {size}")]
    BufferTooSmall { needed: usize, size: usize },

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::string::FromUtf8Error> for AppError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        AppError::WasmGuest(format!("Invalid UTF-8: {}", e))
    }
}

/// Convert AppError to negative i32, for host functions
impl AppError {
    pub fn to_error_code(&self) -> i32 {
        match self {
            AppError::KeyNotFound(_) => -1,
            AppError::InvalidInput(_) => -2,
            AppError::BufferTooSmall { .. } => -3,
            AppError::OutOfMemory => -4,
            AppError::PermissionDenied => -5,
            AppError::Io(_)
            | AppError::Wasm(_)
            | AppError::WasmGuest(_)
            | AppError::Internal(_) => -99,
        }
    }
}

/// Convert i32 error code to string, for logging
pub fn error_code_to_string(code: i32) -> &'static str {
    match code {
        -1 => "Key not found",
        -2 => "Invalid input",
        -3 => "Buffer too small",
        -4 => "Out of memory",
        -5 => "Permission denied",
        -99 => "Internal error",
        _ => "Unknown error",
    }
}
