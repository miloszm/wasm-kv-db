use crate::Storage;
use crate::error::AppError;
use wasmtime::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub struct WasmState {
    pub storage: Storage,
}

pub struct WasmGuest {
    store: Store<WasmState>,
    memory: Memory,
    execute: TypedFunc<(u32, u32), u32>, // input len -> output len
    arg_buf_ofs: usize,
}

impl WasmGuest {
    pub fn new(wasm_bytes: &[u8], storage: Storage) -> Result<Self, AppError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let mut store = Store::new(&engine, WasmState { storage });

        let mut linker = Linker::new(&engine);

        linker
            .func_wrap("env", "host_put", WasmGuest::host_put)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_put: {}", e)))?;

        linker
            .func_wrap("env", "host_get", WasmGuest::host_get)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_get: {}", e)))?;

        let instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| AppError::WasmGuest("guest must export memory".to_string()))?;

        let execute = instance
            .get_typed_func::<(u32, u32), u32>(&mut store, "execute")
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

    pub fn execute(&mut self, name: &[u8], args: &[u8]) -> Result<Vec<u8>, AppError> {
        let input_len = name.len() as u32;
        let args_len = args.len() as u32;
        let name_ptr = self.arg_buf_ofs;
        let argbuf_ptr = self.arg_buf_ofs + input_len as usize;

        // Check buffer capacity
        if input_len + args_len > 65536 {
            return Err(AppError::WasmGuest(format!(
                "input too large: {} bytes (max 65536)",
                name.len()
            )));
        }

        self.memory
            .write(&mut self.store, name_ptr, name)
            .map_err(|e| AppError::WasmGuest(format!("failed to write input: {}", e)))?;

        self.memory
            .write(&mut self.store, argbuf_ptr, args)
            .map_err(|e| AppError::WasmGuest(format!("failed to write input: {}", e)))?;

        let output_len = self
            .execute
            .call(&mut self.store, (input_len, args_len))
            .map_err(|e| AppError::WasmGuest(format!("transform failed: {}", e)))?;

        if output_len as usize > 65536 {
            return Err(AppError::WasmGuest(format!(
                "output too large: {} bytes (max 65536)",
                output_len
            )));
        }

        let mut output = vec![0u8; output_len as usize];
        self.memory
            .read(&mut self.store, name_ptr, &mut output)
            .map_err(|e| AppError::WasmGuest(format!("failed to read output: {}", e)))?;

        Ok(output)
    }

    /// puts key, value into host's KV store
    /// returns 0 on success
    fn host_put(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        // Get memory from the instance
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => return Ok(1), // Error: no memory export
        };

        // key
        let mut key_bytes = vec![0u8; key_len as usize];
        memory.read(&mut caller, key_ptr as usize, &mut key_bytes)?;
        let key =
            String::from_utf8(key_bytes).map_err(|_| wasmtime::Error::msg("invalid UTF-8 key"))?;

        // value
        let mut value_bytes = vec![0u8; value_len as usize];
        memory.read(&mut caller, value_ptr as usize, &mut value_bytes)?;

        let storage = &caller.data().storage;
        match storage.put(&key, value_bytes) {
            Ok(_) => Ok(0),
            Err(_) => Ok(1),
        }
    }

    /// gets value from host store
    /// returns 0 if not found, otherwise length of the found value
    fn host_get(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => return Ok(0),
        };

        // key
        let mut key_bytes = vec![0u8; key_len as usize];
        memory.read(&mut caller, key_ptr as usize, &mut key_bytes)?;
        let key =
            String::from_utf8(key_bytes).map_err(|_| wasmtime::Error::msg("invalid UTF-8 key"))?;

        // get value from storage
        let storage = &caller.data().storage;
        let value = match storage.get(&key) {
            Ok(v) => v,
            Err(_) => return Ok(0),
        };

        let write_len = value.len().min(value_len as usize);
        memory.write(&mut caller, value_ptr as usize, &value[..write_len])?;

        Ok(value.len() as i32)
    }
}
