// cargo build --target wasm32-unknown-unknown

use std::alloc::{Layout, alloc as std_alloc, dealloc as std_dealloc};

#[unsafe(no_mangle)]
static mut ARG_BUF: [u8; 65536] = [0; 65536];

#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: u32) -> *mut u8 {
    let layout = Layout::from_size_align(size as usize, 8).unwrap();
    unsafe { std_alloc(layout) }
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
    if !ptr.is_null() {
        unsafe {
            let layout = Layout::from_size_align(1, 8).unwrap();
            std_dealloc(ptr, layout);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn transform(input_ptr: *const u8, input_len: usize) -> i32 {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };

    // example: convert to uppercase
    let output = input
        .iter()
        .map(|b| b.to_ascii_uppercase())
        .collect::<Vec<u8>>();

    let output_ptr = &raw mut ARG_BUF as *mut u8;

    // copy output to the argument buffer
    unsafe {
        std::ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len());
    }

    output.len() as i32
}
