use anyhow::Result;
use wasmtime::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub struct WasmGuest {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,            // size -> allocated mem ptr
    free: TypedFunc<u32, ()>,              // mem to free ptr -> ()
    transform: TypedFunc<(u32, u32), u32>, // input ptr, input len -> output ptr
    get_output_len: TypedFunc<(), u32>,    // () -> len of the last output
}

impl WasmGuest {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;

        let mut store = Store::new(&engine, ());

        let mut linker = Linker::new(&engine);

        let instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("guest must export memory"))?;

        let alloc = instance.get_typed_func::<u32, u32>(&mut store, "alloc")?;
        let free = instance.get_typed_func::<u32, ()>(&mut store, "free")?;
        let transform = instance.get_typed_func::<(u32, u32), u32>(&mut store, "transform")?;
        let get_output_len = instance.get_typed_func::<(), u32>(&mut store, "get_output_len")?;

        Ok(Self {
            store,
            memory,
            alloc,
            free,
            transform,
            get_output_len,
        })
    }

    pub fn transform(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        let input_len = input.len() as u32;
        let input_ptr = self.alloc.call(&mut self.store, input_len)?;

        self.memory
            .write(&mut self.store, input_ptr as usize, input)?;

        let output_ptr = self
            .transform
            .call(&mut self.store, (input_ptr, input_len))?;

        let output_len = self.get_output_len.call(&mut self.store, ())?;

        let mut output = vec![0u8; output_len as usize];
        self.memory
            .read(&mut self.store, output_ptr as usize, &mut output)?;

        self.free.call(&mut self.store, input_ptr)?;

        Ok(output)
    }
}
