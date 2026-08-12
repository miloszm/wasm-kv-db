mod common;

use common::load_guest;
use wasm_kv_db::{AppError, Storage};

#[test]
pub fn test_wasm_raffle_guest() -> Result<(), AppError> {
    let storage = Storage::new();
    let mut guest = load_guest(storage.clone(), "raffle");

    let name = b"raffle".to_vec();
    let data = b"".to_vec();

    let output = guest.execute(&name, &data)?;
    let output_name = String::from_utf8_lossy(&output).to_string();
    assert_eq!(output_name, "raffle");

    Ok(())
}
