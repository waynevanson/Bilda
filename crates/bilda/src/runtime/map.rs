#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::ffi::c_char;
use std::ptr;

use crate::runtime::alloc::{allocate, copy_name, free_map_entries, free_name};
use crate::runtime::refcount::{bilda_decref, bilda_incref};
use crate::runtime::value::{MapEntry, MapObj, Value, TAG_MAP};
use crate::runtime::value_ops::alloc_value;

pub unsafe fn free_map_obj(obj: *mut MapObj) {
    if obj.is_null() {
        return;
    }
    for i in 0..(*obj).len {
        let entry = &*(*obj).entries.add(i);
        free_name(entry.name);
        bilda_decref(entry.value);
    }
    free_map_entries((*obj).entries, (*obj).capacity);
    free_name((*obj).name);
    dealloc(obj as *mut u8, Layout::new::<MapObj>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_alloc_map(name_data: *const c_char, name_len: usize) -> *mut Value {
    let name = if name_len > 0 {
        copy_name(name_data, name_len)
    } else {
        ptr::null()
    };

    let obj = allocate(Layout::new::<MapObj>()) as *mut MapObj;

    let capacity = 4usize;
    let entries_layout = Layout::array::<MapEntry>(capacity).unwrap_unchecked();
    let entries = allocate(entries_layout) as *mut MapEntry;

    ptr::write(
        obj,
        MapObj {
            refcount: 1,
            name,
            len: 0,
            capacity,
            entries,
        },
    );

    alloc_value(TAG_MAP, obj as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_map_rename(
    map: *mut Value,
    name_data: *const c_char,
    name_len: usize,
) {
    let obj = (*map).payload as *mut MapObj;
    free_name((*obj).name);
    (*obj).name = copy_name(name_data, name_len);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_map_set(
    map: *mut Value,
    name_data: *const c_char,
    name_len: usize,
    value: *mut Value,
) {
    let obj = (*map).payload as *mut MapObj;
    if (*obj).len == (*obj).capacity {
        let old_capacity = (*obj).capacity;
        let new_capacity = old_capacity * 2;
        let new_layout = Layout::array::<MapEntry>(new_capacity).unwrap_unchecked();
        let new_entries = allocate(new_layout) as *mut MapEntry;
        ptr::copy_nonoverlapping((*obj).entries, new_entries, (*obj).len);
        free_map_entries((*obj).entries, old_capacity);
        (*obj).entries = new_entries;
        (*obj).capacity = new_capacity;
    }

    let name = copy_name(name_data, name_len);
    let entry = (*obj).entries.add((*obj).len);
    ptr::write(entry, MapEntry { name, value });
    bilda_incref(value);
    (*obj).len += 1;
}
