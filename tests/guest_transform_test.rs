mod common;
use common::load_guest;
use wasm_kv_db::{AppError, Storage};

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let mut guest = load_guest();

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
