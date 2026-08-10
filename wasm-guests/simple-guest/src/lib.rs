// cargo build --target wasm32-unknown-unknown

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; 65536] = [0; 65536];

#[unsafe(no_mangle)]
pub extern "C" fn transform(input_len: usize) -> i32 {
    let output_ptr = &raw mut ARG_BUF as *mut u8;

    let input = unsafe { std::slice::from_raw_parts(output_ptr, input_len) };

    // example: convert to uppercase
    let output = input
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .collect::<Vec<u8>>();

    unsafe {
        std::ptr::copy(output.as_ptr(), output_ptr, output.len());
    }

    output.len() as i32
}
