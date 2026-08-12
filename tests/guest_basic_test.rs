mod common;

use common::load_guest;
use wasm_kv_db::{AppError, Storage};

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let storage = Storage::new();
    let mut guest = load_guest(storage.clone());

    let input = b"t01:k1".to_vec();

    storage.put("t01:k2", b"abbacafe".to_vec())?;
    // execute will read value from k2 and write it under k1
    // input will be ignored
    let output = guest.execute(&input)?;
    let stored = storage.get("t01:k1")?;

    println!("Input:  {}", String::from_utf8_lossy(&input));
    println!("Output: {:?}", hex::encode(output));
    println!("Stored:  {}", String::from_utf8_lossy(&stored));

    assert_eq!(storage.get("t01:k1")?, b"abbacafe".to_vec());
    assert_eq!(storage.get("t01:k2")?, b"abbacafe".to_vec());

    Ok(())
}
