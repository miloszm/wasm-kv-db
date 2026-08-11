use serde_json::json;
use wasm_kv_db::{AppError, Storage, WasmGuest};

#[test]
fn test_storage_with_wasm_transform() -> Result<(), AppError> {
    // Load the Wasm guest
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;
    let mut guest = WasmGuest::new(&wasm_bytes)?;

    let storage = Storage::new();

    // Insert a value with sensitive fields
    let key = "employee:123";
    let value = json!({
        "name": "Alice",
        "department": "Engineering",
    });
    storage.put(key, value).unwrap();

    // Retrieve without transform (raw)
    let raw = storage.get_raw(key).unwrap();
    assert_eq!(raw["name"], "Alice");
    assert_eq!(raw["department"], "Engineering");

    // Retrieve with transform (redacted)
    let transformed = storage.get_transformed(key, &mut guest).unwrap();
    assert_eq!(transformed["NAME"], "ALICE");
    assert_eq!(transformed["DEPARTMENT"], "ENGINEERING");

    Ok(())
}

#[test]
fn test_storage_wasm_transform_nonexistent() -> Result<(), AppError> {
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;
    let mut guest = WasmGuest::new(&wasm_bytes)?;
    let storage = Storage::new();

    let result = storage.get_transformed("nonexistent", &mut guest);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, wasm_kv_db::AppError::KeyNotFound(_)));

    Ok(())
}
