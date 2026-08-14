mod common;

use common::load_guest;
use rmp_serde::{Deserializer, Serializer};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use wasm_kv_db::{AppError, Storage, WasmGuest};

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

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    value
        .serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| AppError::Serialization(e.to_string()))?;
    Ok(buf)
}

pub fn from_msgpack<T: DeserializeOwned>(data: &[u8]) -> Result<T, AppError> {
    let mut deserializer = Deserializer::new(Cursor::new(data));
    T::deserialize(&mut deserializer).map_err(|e| AppError::Serialization(e.to_string()))
}

#[test]
pub fn raffle_guest_incorrect_target() -> Result<(), AppError> {
    let storage = Storage::new();
    let mut guest = load_guest(storage.clone(), "raffle");

    let name = b"raffle".to_vec();
    let data = b"".to_vec();

    let result = guest.execute(&name, &data);
    assert!(result.is_err());
    Ok(())
}

const TOTAL_TICKETS: u32 = 3;
const RAFFLE_ID: &str = "raffle1";
const USER_ID: &str = "admin";
const USER_ID_SIZE: usize = 8;

fn create_raffle(guest: &mut WasmGuest) -> Result<CreateRaffleResult, AppError> {
    let create_raffle = b"create_raffle".to_vec();
    let create_raffle_args = CreateRaffleArgs {
        raffle_id: RAFFLE_ID.to_string(),
        total_tickets: TOTAL_TICKETS,
        end_time: 0,
    };

    let create_raffle_bytes = to_msgpack(&create_raffle_args)?;
    let result_bytes = guest.execute(&create_raffle, &create_raffle_bytes)?;
    Ok(from_msgpack(&result_bytes)?)
}

#[test]
pub fn raffle_guest_create_raffle() -> Result<(), AppError> {
    let mut storage = Storage::new();
    let mut guest = load_guest(storage.clone(), "raffle");
    let create_raffle_result: CreateRaffleResult = create_raffle(&mut guest)?;

    assert_eq!(create_raffle_result.success, true);
    assert_eq!(
        storage.get_int(format!("raffle:{RAFFLE_ID}:tickets_left").as_str())?,
        TOTAL_TICKETS as i64
    );
    assert!(
        storage
            .get(format!("raffle:{RAFFLE_ID}:entries").as_str())
            .is_ok()
    );
    Ok(())
}

fn buy_ticket(guest: &mut WasmGuest) -> Result<BuyTicketResult, AppError> {
    let buy_ticket_args = BuyTicketArgs {
        raffle_id: RAFFLE_ID.to_string(),
        user_id: USER_ID.to_string(),
        quantity: 1,
    };
    let buy_ticket_bytes = to_msgpack(&buy_ticket_args)?;
    let buy_ticket = b"buy_ticket".to_vec();
    let result_bytes = guest.execute(&buy_ticket, &buy_ticket_bytes)?;
    Ok(from_msgpack(&result_bytes)?)
}

#[test]
pub fn raffle_guest_buy_ticket() -> Result<(), AppError> {
    let mut storage = Storage::new();
    let mut guest = load_guest(storage.clone(), "raffle");
    let _ = create_raffle(&mut guest)?;
    let buy_ticket_result: BuyTicketResult = buy_ticket(&mut guest)?;

    assert_eq!(buy_ticket_result.tickets_left, 2);
    assert_eq!(buy_ticket_result.success, true);

    assert_eq!(
        storage.get_int(format!("raffle:{RAFFLE_ID}:tickets_left").as_str())?,
        (TOTAL_TICKETS - 1) as i64
    );
    let entries = storage.get(format!("raffle:{RAFFLE_ID}:entries").as_str())?;
    let chunks = entries.chunks(USER_ID_SIZE);
    assert_eq!(
        chunks.last().expect("chunk should exist"),
        USER_ID.as_bytes()
    );

    Ok(())
}
