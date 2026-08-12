use dlmalloc::GlobalDlmalloc;
use serde::{Deserialize, Serialize};
use rmp_serde::{Serializer, Deserializer};
use serde::de::DeserializeOwned;
use std::io::Cursor;

#[global_allocator]
static ALLOC: GlobalDlmalloc = GlobalDlmalloc;

const ARG_BUF_SIZE: usize = 65536;

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; ARG_BUF_SIZE] = [0; ARG_BUF_SIZE];

unsafe extern "C" {
    fn host_put(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
    fn host_get(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRaffleArgs {
    pub raffle_id: String,
    pub total_tickets: u32,
    pub end_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReducerError {
    UnknownReducer,
    SerializationError,
    InsufficientTickets,
    RaffleNotFound,
    RaffleAlreadyEnded,
    NoEntries,
    Unauthorized,
    InternalError,
}

impl std::fmt::Display for ReducerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, ReducerError> {
    let mut buf = Vec::new();
    value
        .serialize(&mut Serializer::new(&mut buf))
        .map_err(|_| ReducerError::SerializationError)?;
    Ok(buf)
}

pub fn from_msgpack<T: DeserializeOwned>(data: &[u8]) -> Result<T, ReducerError> {
    let mut deserializer = Deserializer::new(Cursor::new(data));
    T::deserialize(&mut deserializer).map_err(|_|ReducerError::SerializationError)
}

#[unsafe(no_mangle)]
pub extern "C" fn execute(input_len: usize) -> i32 {
    let buf_ptr = &raw mut ARG_BUF as *mut u8;

    let input_key = unsafe { std::slice::from_raw_parts(buf_ptr, input_len) }.to_vec();

    let k2_key = b"t01:k2".to_vec();

    // read value from "t01:k2" and store it under "t01:k1"
    let k2_value_len = unsafe { host_get(k2_key.as_ptr(), k2_key.len(), buf_ptr, ARG_BUF_SIZE) };
    assert!(k2_value_len > 0);

    // we have ARG_BUF filled out with our k2 value
    let result = unsafe {
        host_put(
            input_key.as_ptr(),
            input_key.len(),
            buf_ptr,
            k2_value_len as usize,
        )
    };

    let output_bytes = result.to_le_bytes();
    unsafe {
        std::ptr::copy(output_bytes.as_ptr(), buf_ptr, 4);
    }

    4
}
