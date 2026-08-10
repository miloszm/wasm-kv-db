use anyhow::{Result, anyhow};
use serde_json::Value;
use wasmtime::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub struct WasmGuest {
    store: Store<()>,
    memory: Memory,
    transform: TypedFunc<u32, u32>, // input len -> output len
    arg_buf_ofs: usize,
}

impl WasmGuest {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let mut store = Store::new(&engine, ());

        let linker = Linker::new(&engine);

        let instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("guest must export memory"))?;

        let transform = instance.get_typed_func::<u32, u32>(&mut store, "transform")?;

        let arg_buf = instance
            .get_global(&mut store, "ARG_BUF")
            .ok_or(anyhow!("argument buffer not found"))?;
        let val = arg_buf.get(&mut store);
        let arg_buf_ofs = val.i32().ok_or(anyhow!("invalid argument buffer"))? as usize;

        Ok(Self {
            store,
            memory,
            transform,
            arg_buf_ofs,
        })
    }

    /// Transforms a JSON value using the Wasm guest
    pub fn transform_json(&mut self, input: &Value) -> Result<Value> {
        let input_bytes = serde_json::to_vec(input)?;
        let output_bytes = self.transform_bytes(&input_bytes)?;
        let output: Value = serde_json::from_slice(&output_bytes)?;
        Ok(output)
    }

    pub fn transform_bytes(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        let argbuf_ptr = self.arg_buf_ofs;
        let input_len = input.len() as u32;

        self.memory
            .write(&mut self.store, argbuf_ptr as usize, input)?;

        let output_len = self.transform.call(&mut self.store, input_len)?;

        let mut output = vec![0u8; output_len as usize];
        self.memory
            .read(&mut self.store, argbuf_ptr as usize, &mut output)?;

        Ok(output)
    }
}
