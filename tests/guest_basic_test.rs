mod common;

use common::load_guest;
use wasm_kv_db::AppError;

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let mut guest = load_guest();

    let input = serde_json::from_str(
        r#"{"department": "Engineering", "name": "Alice", "personal_email": "alice@gmail.com", "salary": 95000}"#,
    )?;

    let output = guest.transform_json(&input)?;

    println!("Input:  {}", input.to_string());
    println!("Output: {}", output.to_string());

    Ok(())
}
