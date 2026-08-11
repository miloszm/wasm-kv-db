use wasm_kv_db::WasmGuest;

pub fn load_guest() -> WasmGuest {
    let wasm_bytes = std::fs::read(
        "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm",
    )
    .expect("Failed to read Wasm file");

    WasmGuest::new(&wasm_bytes).expect("Failed to instantiate Wasm guest")
}
