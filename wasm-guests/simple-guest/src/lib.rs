// cargo build --target wasm32-unknown-unknown

use std::alloc::{Layout, alloc as std_alloc, dealloc as std_dealloc};

static mut ALLOCATION: (*mut u8, usize) = (std::ptr::null_mut(), 0);

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
pub extern "C" fn set_output(ptr: *mut u8, len: usize) {
    unsafe { ALLOCATION = (ptr, len) }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_output_len() -> usize {
    unsafe { ALLOCATION.1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn transform(input_ptr: *const u8, input_len: usize) -> *mut u8 {
    let input = unsafe {
        std::slice::from_raw_parts(input_ptr, input_len)
    };

    // example: convert to uppercase
    let output = input.iter().map(|b| b.to_ascii_uppercase()).collect::<Vec<u8>>();

    let output_ptr = alloc(output.len() as u32);

    // copy output to allocated memory
    unsafe {
        std::ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len());
    }

    set_output(output_ptr, output.len());

    output_ptr
}