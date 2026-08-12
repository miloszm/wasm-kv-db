// cargo build --target wasm32-unknown-unknown

const ARG_BUF_SIZE: usize = 65536;

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; ARG_BUF_SIZE] = [0; ARG_BUF_SIZE];

unsafe extern "C" {
    fn host_put(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
    fn host_get(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn execute(input_len: usize) -> i32 {
    let buf_ptr = &raw mut ARG_BUF as *mut u8;

    let _input = unsafe { std::slice::from_raw_parts(buf_ptr, input_len) };

    let k1_key = b"t01:k1".to_vec();
    let k2_key = b"t01:k2".to_vec();

    // read value from "t01:k2" and store it under "t01:k1"
    let k2_value_len = unsafe {
        host_get(k2_key.as_ptr(), k2_key.len(), buf_ptr, ARG_BUF_SIZE)
    };
    assert!(k2_value_len > 0);

    // we have ARG_BUF filled out with our k2 value
    let result = unsafe { host_put(k1_key.as_ptr(), k1_key.len(), buf_ptr, k2_value_len as usize) };

    let output_bytes = result.to_le_bytes();
    unsafe {
        std::ptr::copy(output_bytes.as_ptr(), buf_ptr, 4);
    }

    4
}
