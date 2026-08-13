use crate::Storage;
use crate::error::AppError;
use wasmtime::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub struct WasmState {
    pub storage: Storage,
    pub user_id: String,
}

impl WasmState {
    pub fn new(storage: Storage, user_id: String) -> Self {
        Self { storage, user_id }
    }
}

pub struct WasmGuest {
    store: Store<WasmState>,
    memory: Memory,
    execute: TypedFunc<(u32, u32), u32>, // input len -> output len
    arg_buf_ofs: usize,
}

impl WasmGuest {
    pub fn new(
        wasm_bytes: &[u8],
        storage: Storage,
        user_id: impl AsRef<str>,
    ) -> Result<Self, AppError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let mut store = Store::new(
            &engine,
            WasmState::new(storage, user_id.as_ref().to_string()),
        );

        let mut linker = Linker::new(&engine);

        linker
            .func_wrap("env", "host_put", WasmGuest::host_put)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_put: {}", e)))?;

        linker
            .func_wrap("env", "host_put_int", WasmGuest::host_put_int)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_put_int: {}", e)))?;

        linker
            .func_wrap("env", "host_get", WasmGuest::host_get)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_get: {}", e)))?;

        linker
            .func_wrap("env", "host_get_len", WasmGuest::host_get_len)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_get_len: {}", e)))?;

        linker
            .func_wrap("env", "host_get_int", WasmGuest::host_get_int)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_get_int: {}", e)))?;

        linker
            .func_wrap("env", "host_caller", WasmGuest::host_caller)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_caller: {}", e)))?;

        linker
            .func_wrap("env", "host_append_to_list", WasmGuest::host_append_to_list)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_append_to_list: {}", e)))?;

        linker
            .func_wrap("env", "host_rand", WasmGuest::host_rand)
            .map_err(|e| AppError::WasmGuest(format!("failed to link host_rand: {}", e)))?;

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
            _ => {
                eprintln!("host_put: memory export not found");
                return Ok(-99);
            }
        };

        let mut key_bytes = vec![0u8; key_len as usize];
        if let Err(e) = memory.read(&mut caller, key_ptr as usize, &mut key_bytes) {
            eprintln!("host_put: failed to read key: {}", e);
            return Ok(-2);
        }

        let key = match String::from_utf8(key_bytes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("host_put: invalid UTF-8 key: {}", e);
                return Ok(-2);
            }
        };

        // value
        let mut value_bytes = vec![0u8; value_len as usize];
        memory.read(&mut caller, value_ptr as usize, &mut value_bytes)?;

        let storage = &caller.data().storage;
        match storage.put(&key, value_bytes) {
            Ok(_) => Ok(0),
            Err(_) => {
                eprintln!("host_put: storage insertion failed");
                Ok(-99)
            }
        }
    }

    /// gets value from host store
    /// returns length of data passed back or negative error code
    fn host_get(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_get: memory export not found");
                return Ok(-99);
            }
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
            Err(_) => {
                eprintln!("host_get: not found");
                return Ok(-1);
            }
        };

        let write_len = value.len().min(value_len as usize);
        memory.write(&mut caller, value_ptr as usize, &value[..write_len])?;

        Ok(value.len() as i32)
    }

    /// gets length of the value from host store
    fn host_get_len(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_get_len: memory export not found");
                return Ok(-99);
            }
        };

        // key
        let mut key_bytes = vec![0u8; key_len as usize];
        memory.read(&mut caller, key_ptr as usize, &mut key_bytes)?;
        let key =
            String::from_utf8(key_bytes).map_err(|_| wasmtime::Error::msg("invalid UTF-8 key"))?;

        // get value from storage
        let storage = &caller.data().storage;
        let len = match storage.get_len(&key) {
            Ok(len) => len,
            Err(_) => {
                eprintln!("host_get_len: not found");
                return Ok(-1);
            }
        };

        Ok(len as i32)
    }

    /// returns user id that is calling the guest
    fn host_caller(
        mut caller: wasmtime::Caller<'_, WasmState>,
        value_ptr: u32,
        value_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_get: memory export not found");
                return Ok(-99);
            }
        };

        let user_id_bytes = caller.data().user_id.as_bytes().to_vec();
        if user_id_bytes.len() > value_len as usize {
            return Ok(-3);
        }
        memory.write(&mut caller, value_ptr as usize, &user_id_bytes)?;
        Ok(user_id_bytes.len() as i32)
    }

    fn host_put_int(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
        value: i64,
    ) -> Result<i32, wasmtime::Error> {
        // Get memory from the instance
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_put: memory export not found");
                return Ok(-99);
            }
        };

        let mut key_bytes = vec![0u8; key_len as usize];
        if let Err(e) = memory.read(&mut caller, key_ptr as usize, &mut key_bytes) {
            eprintln!("host_put_int: failed to read key: {}", e);
            return Ok(-2); // InvalidInput
        }

        let key = match String::from_utf8(key_bytes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("host_put_int: invalid UTF-8 key: {}", e);
                return Ok(-2);
            }
        };

        let value_bytes = value.to_le_bytes().to_vec();

        let storage = &caller.data().storage;
        match storage.put(&key, value_bytes) {
            Ok(_) => Ok(0),
            Err(e) => {
                eprintln!("host_put_int: storage insertion failed");
                Ok(e.to_error_code())
            }
        }
    }

    fn host_get_int(
        mut caller: wasmtime::Caller<'_, WasmState>,
        key_ptr: u32,
        key_len: u32,
    ) -> Result<i64, wasmtime::Error> {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_get_int: memory export not found");
                return Ok(-99); // Internal error
            }
        };

        let mut key_bytes = vec![0u8; key_len as usize];
        if let Err(e) = memory.read(&mut caller, key_ptr as usize, &mut key_bytes) {
            eprintln!("host_get_int: failed to read key: {}", e);
            return Ok(-2); // InvalidInput
        }

        let key = match String::from_utf8(key_bytes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("host_get_int: invalid UTF-8 key: {}", e);
                return Ok(-2); // InvalidInput
            }
        };

        // Get value from storage
        let storage = &caller.data().storage;
        match storage.get(&key) {
            Ok(value_bytes) => {
                if value_bytes.len() != 8 {
                    eprintln!(
                        "host_get_int: value for key '{}' is not an i64 (len={})",
                        key,
                        value_bytes.len()
                    );
                    return Ok(-2); // InvalidInput
                }

                let value = i64::from_le_bytes(
                    value_bytes.try_into().unwrap(), // Safe because we checked len == 8
                );
                Ok(value)
            }
            Err(AppError::KeyNotFound(_)) => {
                eprintln!("host_get_int: key '{}' not found", key);
                Ok(-1) // KeyNotFound
            }
            Err(e) => {
                eprintln!("host_get_int: storage error for key '{}': {}", key, e);
                Ok(e.to_error_code() as i64)
            }
        }
    }

    /// adds entry to an existing list
    /// returns 0 on success
    fn host_append_to_list(
        mut caller: wasmtime::Caller<'_, WasmState>,
        entries_key_ptr: u32,
        entries_key_len: u32,
        entry_ptr: u32,
        entry_len: u32,
    ) -> Result<i32, wasmtime::Error> {
        // Get memory from the instance
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(mem)) => mem,
            _ => {
                eprintln!("host_put: memory export not found");
                return Ok(-99);
            }
        };

        let mut key_bytes = vec![0u8; entries_key_len as usize];
        if let Err(e) = memory.read(&mut caller, entries_key_ptr as usize, &mut key_bytes) {
            eprintln!("host_append_to_list: failed to read key: {}", e);
            return Ok(-2);
        }

        let key = match String::from_utf8(key_bytes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("host_append_to_list: invalid UTF-8 key: {}", e);
                return Ok(-2);
            }
        };

        // entry
        let mut entry_bytes = vec![0u8; entry_len as usize];
        memory.read(&mut caller, entry_ptr as usize, &mut entry_bytes)?;

        let storage = &caller.data().storage;
        match storage.append_to_list(&key, entry_bytes) {
            Ok(_) => Ok(0),
            Err(_) => {
                eprintln!("host_put: storage insertion failed");
                Ok(-99)
            }
        }
    }

    /// returns random number in range 0..max
    fn host_rand(
        _caller: wasmtime::Caller<'_, WasmState>,
        max: u32,
    ) -> Result<u32, wasmtime::Error> {
        if max == 0 {
            eprintln!("host_rand: max must be > 0");
            return Ok(0);
        }
        let random = rand::random_range(0..max);
        Ok(random)
    }
}
