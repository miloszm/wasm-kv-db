mod common;

use common::load_guest;
use wasm_kv_db::{AppError, Storage};

#[test]
pub fn test_wasm_basic_guest() -> Result<(), AppError> {
    let storage = Storage::new();
    let mut guest = load_guest(storage.clone(), "simple");

    let name = b"guest".to_vec();
    let input_key = b"t01:k1".to_vec();

    storage.put("t01:k2", b"abbacafe".to_vec())?;
    // execute will read value from k2 and write it under input_key
    let output = guest.execute(&name, &input_key)?;
    let stored = storage.get("t01:k1")?;

    println!("Input:  {}", String::from_utf8_lossy(&input_key));
    println!("Output: {:?}", hex::encode(output));
    println!("Stored:  {}", String::from_utf8_lossy(&stored));

    assert_eq!(storage.get("t01:k1")?, b"abbacafe".to_vec());
    assert_eq!(storage.get("t01:k2")?, b"abbacafe".to_vec());

    Ok(())
}
