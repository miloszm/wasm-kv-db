mod common;

use common::load_guest;
use wasm_kv_db::{AppError, Storage};

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let storage = Storage::new();
    let mut guest = load_guest(storage.clone());

    let input = b"t01:k1".to_vec();

    let output = guest.execute(&input)?;

    let stored = storage.get("t01:k1")?;

    println!("Input:  {}", String::from_utf8_lossy(&input));
    println!("Output: {:?}", hex::encode(output));
    println!("Stored:  {}", String::from_utf8_lossy(&stored));

    Ok(())
}
