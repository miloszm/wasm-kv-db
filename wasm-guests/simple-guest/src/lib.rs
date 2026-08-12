// cargo build --target wasm32-unknown-unknown

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; 65536] = [0; 65536];

unsafe extern "C" {
    fn host_put(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn execute(input_len: usize) -> i32 {
    let buf_ptr = &raw mut ARG_BUF as *mut u8;

    let _input = unsafe { std::slice::from_raw_parts(buf_ptr, input_len) };

    let value = b"cafebabe";

    // write the value to ARG_BUF after the input
    let value_ptr = unsafe { buf_ptr.add(input_len) };
    unsafe {
        std::ptr::copy(value.as_ptr(), value_ptr, value.len());
    }
    let result = unsafe { host_put(buf_ptr, input_len, value_ptr, value.len()) };

    let output_bytes = result.to_le_bytes();
    unsafe {
        std::ptr::copy(output_bytes.as_ptr(), buf_ptr, 4);
    }

    4
}
