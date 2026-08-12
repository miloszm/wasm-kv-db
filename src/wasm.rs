use crate::error::AppError;
use wasmtime::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub struct WasmGuest {
    store: Store<()>,
    memory: Memory,
    execute: TypedFunc<u32, u32>, // input len -> output len
    arg_buf_ofs: usize,
}

impl WasmGuest {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, AppError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let mut store = Store::new(&engine, ());

        let linker = Linker::new(&engine);

        let instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| AppError::WasmGuest("guest must export memory".to_string()))?;

        let execute = instance
            .get_typed_func::<u32, u32>(&mut store, "execute")
            .map_err(|e| AppError::WasmGuest(format!("failed to get execute: {}", e)))?;

        let arg_buf = instance
            .get_global(&mut store, "ARG_BUF")
            .ok_or_else(|| AppError::WasmGuest("argument buffer not found".to_string()))?;
        let val = arg_buf.get(&mut store);
        let arg_buf_ofs = val.i32().ok_or_else(|| {
            AppError::WasmGuest("invalid argument buffer type (expected i32)".to_string())
        })? as usize;

        Ok(Self {
            store,
            memory,
            execute,
            arg_buf_ofs,
        })
    }

    pub fn execute(&mut self, input: &[u8]) -> Result<Vec<u8>, AppError> {
        let argbuf_ptr = self.arg_buf_ofs;
        let input_len = input.len() as u32;

        // Check buffer capacity
        if input.len() > 65536 {
            return Err(AppError::WasmGuest(format!(
                "input too large: {} bytes (max 65536)",
                input.len()
            )));
        }

        self.memory
            .write(&mut self.store, argbuf_ptr, input)
            .map_err(|e| AppError::WasmGuest(format!("failed to write input: {}", e)))?;

        let output_len = self
            .execute
            .call(&mut self.store, input_len)
            .map_err(|e| AppError::WasmGuest(format!("transform failed: {}", e)))?;

        if output_len as usize > 65536 {
            return Err(AppError::WasmGuest(format!(
                "output too large: {} bytes (max 65536)",
                output_len
            )));
        }

        let mut output = vec![0u8; output_len as usize];
        self.memory
            .read(&mut self.store, argbuf_ptr, &mut output)
            .map_err(|e| AppError::WasmGuest(format!("failed to read output: {}", e)))?;

        Ok(output)
    }
}
