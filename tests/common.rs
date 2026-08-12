use wasm_kv_db::{Storage, WasmGuest};

pub fn load_guest(storage: Storage, name: &str) -> WasmGuest {
    let wasm_path = match name {
        "simple" => {
            "wasm-guests/simple-guest/target/wasm32-unknown-unknown/debug/simple_guest.wasm"
        }
        "raffle" => "wasm-guests/raffle/target/wasm32-unknown-unknown/debug/raffle.wasm",
        _ => panic!("Guest name not found"),
    };
    let wasm_bytes = std::fs::read(wasm_path).expect("Failed to read Wasm file");

    WasmGuest::new(&wasm_bytes, storage, "admin").expect("Failed to instantiate Wasm guest")
}
