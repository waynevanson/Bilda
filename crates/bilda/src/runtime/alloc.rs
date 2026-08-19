#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::ffi::{CStr, c_char};
use std::ptr;

use crate::runtime::value::MapEntry;

pub unsafe fn allocate(layout: Layout) -> *mut u8 {
    let ptr = alloc(layout);
    if ptr.is_null() {
        handle_alloc_error(layout);
    }
    ptr
}

pub unsafe fn copy_name(data: *const c_char, len: usize) -> *const c_char {
    let layout = Layout::from_size_align_unchecked(len + 1, 1);
    let ptr = allocate(layout) as *mut c_char;
    if len > 0 {
        ptr::copy_nonoverlapping(data, ptr, len);
    }
    *ptr.add(len) = 0;
    ptr
}

pub unsafe fn free_name(name: *const c_char) {
    if name.is_null() {
        return;
    }
    let len = CStr::from_ptr(name).to_bytes().len();
    let layout = Layout::from_size_align_unchecked(len + 1, 1);
    dealloc(name as *mut u8, layout);
}

pub unsafe fn free_map_entries(entries: *mut MapEntry, capacity: usize) {
    if entries.is_null() || capacity == 0 {
        return;
    }
    let layout = Layout::array::<MapEntry>(capacity).unwrap_unchecked();
    dealloc(entries as *mut u8, layout);
}
