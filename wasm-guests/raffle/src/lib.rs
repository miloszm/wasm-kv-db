use dlmalloc::GlobalDlmalloc;
use rmp_serde::{Deserializer, Serializer};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[global_allocator]
static ALLOC: GlobalDlmalloc = GlobalDlmalloc;

const ARG_BUF_SIZE: usize = 65536;

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; ARG_BUF_SIZE] = [0; ARG_BUF_SIZE];

#[allow(unused)]
unsafe extern "C" {
    fn host_put(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
    fn host_put_int(key_ptr: *const u8, key_len: usize, value: i64) -> i32;
    fn host_get(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
    fn host_get_len(key_ptr: *const u8, key_len: usize) -> i32;
    fn host_get_int(key_ptr: *const u8, key_len: usize) -> i64;
    fn host_append_to_list(
        key_ptr: *const u8,
        key_len: usize,
        value_ptr: *const u8,
        value_len: usize,
    ) -> ();
    fn host_caller(caller_ptr: *const u8, caller_len: usize) -> i32;
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestError {
    Success = 0,
    KeyNotFound = -1,
    InvalidInput = -2,
    BufferTooSmall = -3,
    OutOfMemory = -4,
    PermissionDenied = -5,
    Internal = -99,
}

impl GuestError {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn from_i32(code: i32) -> Self {
        match code {
            -1 => GuestError::KeyNotFound,
            -2 => GuestError::InvalidInput,
            -3 => GuestError::BufferTooSmall,
            -4 => GuestError::OutOfMemory,
            -5 => GuestError::PermissionDenied,
            -99 => GuestError::Internal,
            _ => GuestError::Internal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRaffleArgs {
    pub raffle_id: String,
    pub total_tickets: u32,
    pub end_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRaffleResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyTicketArgs {
    pub raffle_id: String,
    pub user_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyTicketResult {
    pub success: bool,
    pub message: String,
    pub tickets_left: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawWinnerArgs {
    pub raffle_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawWinnerResult {
    pub success: bool,
    pub winner: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReducerError {
    UnknownReducer,
    SerializationError,
    InsufficientTickets,
    RaffleNotFound,
    RaffleAlreadyEnded,
    NoEntries,
    InvalidEntries,
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
    T::deserialize(&mut deserializer).map_err(|_| ReducerError::SerializationError)
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

fn execute_create_raffle(args_bytes: &[u8]) -> Result<Vec<u8>, ReducerError> {
    let args: CreateRaffleArgs = from_msgpack(args_bytes)?;
    let result = create_raffle(args)?;
    to_msgpack(&result)
}

fn get_caller() -> Result<String, ReducerError> {
    const CALLER_LEN: usize = 256;
    let mut caller_buf = vec![0u8; CALLER_LEN];

    let caller_len = unsafe { host_caller(caller_buf.as_mut_ptr(), CALLER_LEN) };
    if caller_len < 0 {
        return Err(ReducerError::Unauthorized); // host_caller failed
    }
    let caller_len = caller_len as usize;

    if caller_len >= CALLER_LEN {
        return Err(ReducerError::Unauthorized);
    }

    let caller = match String::from_utf8(caller_buf[..caller_len].to_vec()) {
        Ok(s) => s,
        Err(_) => return Err(ReducerError::Unauthorized),
    };

    Ok(caller)
}

fn create_raffle(args: CreateRaffleArgs) -> Result<CreateRaffleResult, ReducerError> {
    let caller = get_caller()?;
    if caller != "admin" {
        return Err(ReducerError::Unauthorized);
    }

    let tickets_key = format!("raffle:{}:tickets_left", args.raffle_id);
    let tickets_bytes = args.total_tickets.to_le_bytes().to_vec();
    unsafe {
        host_put(
            tickets_key.as_ptr(),
            tickets_key.len(),
            tickets_bytes.as_ptr(),
            tickets_bytes.len(),
        );
    }

    let end_time_key = format!("raffle:{}:end_time", args.raffle_id);
    let end_time_bytes = args.end_time.to_le_bytes().to_vec();
    unsafe {
        host_put(
            end_time_key.as_ptr(),
            end_time_key.len(),
            end_time_bytes.as_ptr(),
            end_time_bytes.len(),
        );
    }

    let entries_key = format!("raffle:{}:entries", args.raffle_id);
    unsafe {
        host_put(entries_key.as_ptr(), entries_key.len(), std::ptr::null(), 0);
    }

    Ok(CreateRaffleResult {
        success: true,
        message: format!(
            "Raffle {} created with {} tickets",
            args.raffle_id, args.total_tickets
        ),
    })
}

fn execute_buy_ticket(args_bytes: &[u8]) -> Result<Vec<u8>, ReducerError> {
    // Deserialize arguments
    let args: BuyTicketArgs = from_msgpack(args_bytes)?;

    // Call the actual business logic
    let result = buy_ticket(args)?;

    // Serialize result
    to_msgpack(&result)
}

fn buy_ticket(args: BuyTicketArgs) -> Result<BuyTicketResult, ReducerError> {
    let caller = get_caller()?;
    if caller != args.user_id {
        return Err(ReducerError::Unauthorized);
    }

    // todo: check raffle time

    let tickets_key = format!("raffle:{}:tickets_left", args.raffle_id);
    let tickets_left = unsafe { host_get_int(tickets_key.as_ptr(), tickets_key.len()) };

    if tickets_left <= 0 {
        return Err(ReducerError::InsufficientTickets);
    }

    if args.quantity > tickets_left as u32 {
        return Err(ReducerError::InsufficientTickets);
    }

    let new_tickets_left = tickets_left - args.quantity as i64;

    // Update tickets_left
    let key_bytes = tickets_key.as_bytes();
    unsafe {
        host_put_int(key_bytes.as_ptr(), key_bytes.len(), new_tickets_left);
    }

    // Append user to entries list
    let entries_key = format!("raffle:{}:entries", args.raffle_id);
    for _ in 0..args.quantity {
        let user_bytes = args.user_id.as_bytes();
        unsafe {
            host_append_to_list(
                entries_key.as_ptr(),
                entries_key.len(),
                user_bytes.as_ptr(),
                user_bytes.len(),
            );
        }
    }

    Ok(BuyTicketResult {
        success: true,
        message: format!(
            "Purchased {} ticket(s)! {} remaining",
            args.quantity, new_tickets_left
        ),
        tickets_left: new_tickets_left as u32,
    })
}

fn execute_draw_winner(args_bytes: &[u8]) -> Result<Vec<u8>, ReducerError> {
    let args: DrawWinnerArgs = from_msgpack(args_bytes)?;
    let result = draw_winner(args)?;
    to_msgpack(&result)
}

fn draw_winner(args: DrawWinnerArgs) -> Result<DrawWinnerResult, ReducerError> {
    const ENTRY_SIZE: usize = 8;
    let entries_key = format!("raffle:{}:entries", args.raffle_id);

    let entries_len = unsafe { host_get_len(entries_key.as_ptr(), entries_key.len()) };
    if entries_len <= 0 {
        return Err(ReducerError::NoEntries);
    }
    let entries_len = entries_len as usize;
    if entries_len % ENTRY_SIZE != 0 {
        return Err(ReducerError::InvalidEntries);
    }

    let mut entries_buf = vec![0u8; entries_len as usize];
    let actual_entries_len = unsafe {
        host_get(
            entries_key.as_ptr(),
            entries_key.len(),
            entries_buf.as_mut_ptr(),
            entries_len,
        )
    };
    if actual_entries_len as usize != entries_len {
        return Err(ReducerError::InvalidEntries);
    }

    let winner = "";
    Ok(DrawWinnerResult {
        success: true,
        winner: Some(winner.into()),
        message: format!("Winner drawn: {}", winner),
    })
}
