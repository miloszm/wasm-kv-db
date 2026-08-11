use wasm_kv_db::{AppError, WasmGuest};

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )?;

    let mut guest = WasmGuest::new(&wasm_bytes)?;

    let input = serde_json::from_str(
        r#"{"department": "Engineering", "name": "Alice", "personal_email": "alice@gmail.com", "salary": 95000}"#,
    )?;

    let output = guest.transform_json(&input)?;

    println!("Input:  {}", input.to_string());
    println!("Output: {}", output.to_string());

    Ok(())
}
