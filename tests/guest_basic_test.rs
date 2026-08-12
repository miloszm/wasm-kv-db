mod common;

use common::load_guest;
use wasm_kv_db::AppError;

#[test]
pub fn test_wasm_guest() -> Result<(), AppError> {
    let mut guest = load_guest();

    let input = vec![0x01, 0x02];

    let output = guest.execute(&input)?;

    println!("Input:  {:?}", input);
    println!("Output: {:?}", hex::encode(output));

    Ok(())
}
