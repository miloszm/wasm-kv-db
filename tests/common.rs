use wasm_kv_db::{Storage, WasmGuest};

pub fn load_guest(storage: Storage) -> WasmGuest {
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )
    .expect("Failed to read Wasm file");

    WasmGuest::new(&wasm_bytes, storage).expect("Failed to instantiate Wasm guest")
}
