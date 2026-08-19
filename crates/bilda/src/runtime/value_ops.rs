#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ptr;

use crate::runtime::alloc::allocate;
use crate::runtime::value::{Value, TAG_BOOL, TAG_FLOAT, TAG_INT};

pub unsafe fn alloc_value(tag: u64, payload: u64) -> *mut Value {
    let layout = std::alloc::Layout::new::<Value>();
    let value = allocate(layout) as *mut Value;
    ptr::write(
        value,
        Value {
            refcount: 1,
            tag,
            payload,
        },
    );
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_int(n: isize) -> *mut Value {
    alloc_value(TAG_INT, n as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_bool(b: bool) -> *mut Value {
    alloc_value(TAG_BOOL, if b { 1 } else { 0 })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_float(f: f64) -> *mut Value {
    alloc_value(TAG_FLOAT, f.to_bits())
}
