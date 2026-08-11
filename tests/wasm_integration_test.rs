mod common;
use common::load_guest;
use serde_json::json;
use wasm_kv_db::{AppError, Storage};

#[test]
fn test_storage_with_wasm_transform() -> Result<(), AppError> {
    let mut guest = load_guest();

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
    let mut guest = load_guest();
    let storage = Storage::new();

    let result = storage.get_transformed("nonexistent", &mut guest);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, wasm_kv_db::AppError::KeyNotFound(_)));

    Ok(())
}
