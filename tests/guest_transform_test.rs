use wasm_kv_db::{AppError, Storage, WasmGuest};

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;

    let mut guest = WasmGuest::new(&wasm_bytes)?;

    let value = serde_json::from_str(
        r#"{"department": "Engineering", "name": "Alice", "personal_email": "alice@gmail.com", "salary": 95000}"#,
    )?;

    let storage = Storage::new();
    storage.put("t01:employee:123", value).unwrap();
    let result = storage.get_transformed("t01:employee:123", &mut guest);
    assert!(result.is_ok());
    println!("Output: {}", result.expect("must exist").to_string());

    Ok(())
}
