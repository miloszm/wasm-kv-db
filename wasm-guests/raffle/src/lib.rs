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

#[allow(unused)]
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
pub extern "C" fn execute(name_len: usize, _args_len: usize) -> i32 {
    let name_ptr = &raw mut ARG_BUF as *mut u8;
    let buf_ptr = unsafe { name_ptr.add(name_len) };

    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len) }.to_vec();
    let name = String::from_utf8(name_bytes).unwrap_or("".to_string());

    let output_bytes = name.as_bytes();
    unsafe {
        std::ptr::copy(output_bytes.as_ptr(), buf_ptr, name_len);
    }

    name_len as i32
}
