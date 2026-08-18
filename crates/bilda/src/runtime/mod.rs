#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::ffi::{CStr, c_void};
use std::os::raw::c_char;
use std::{ptr, slice};

use crate::runtime::value::{
    BildaFn, ClosureObj, MapEntry, MapObj, StringObj, TAG_BOOL, TAG_FLOAT, TAG_FUNCTION, TAG_INT,
    TAG_MAP, TAG_STRING, Value,
};

pub mod value;

pub use crate::runtime::value::{BildaFn as FunctionPointer, Value as RawValue};

unsafe fn allocate(layout: Layout) -> *mut u8 {
    let ptr = alloc(layout);
    if ptr.is_null() {
        handle_alloc_error(layout);
    }
    ptr
}

unsafe fn copy_name(data: *const c_char, len: usize) -> *const c_char {
    let layout = Layout::from_size_align_unchecked(len + 1, 1);
    let ptr = allocate(layout) as *mut c_char;
    if len > 0 {
        ptr::copy_nonoverlapping(data, ptr, len);
    }
    *ptr.add(len) = 0;
    ptr
}

unsafe fn alloc_value(tag: u64, payload: u64) -> *mut Value {
    let layout = Layout::new::<Value>();
    let value = allocate(layout) as *mut Value;
    ptr::write(value, Value {
        refcount: 1,
        tag,
        payload,
    });
    value
}

unsafe fn string_layout(len: usize) -> Layout {
    let size = size_of::<StringObj>() + len + 1;
    let align = align_of::<StringObj>();
    let padded = (size + align - 1) & !(align - 1);
    Layout::from_size_align_unchecked(padded, align)
}

unsafe fn free_string_obj(obj: *mut StringObj) {
    let layout = string_layout((*obj).len);
    dealloc(obj as *mut u8, layout);
}

unsafe fn free_map_entries(entries: *mut MapEntry, capacity: usize) {
    if entries.is_null() || capacity == 0 {
        return;
    }
    let layout = Layout::array::<MapEntry>(capacity).unwrap_unchecked();
    dealloc(entries as *mut u8, layout);
}

unsafe fn free_name(name: *const c_char) {
    if name.is_null() {
        return;
    }
    let len = CStr::from_ptr(name).to_bytes().len();
    let layout = Layout::from_size_align_unchecked(len + 1, 1);
    dealloc(name as *mut u8, layout);
}

unsafe fn free_map_obj(obj: *mut MapObj) {
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

unsafe fn free_closure_obj(obj: *mut ClosureObj) {
    if obj.is_null() {
        return;
    }
    // The environment pointer is opaque and currently always null in JIT output.
    dealloc(obj as *mut u8, Layout::new::<ClosureObj>());
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_string(data: *const c_char, len: usize) -> *mut Value {
    let layout = string_layout(len);
    let obj = allocate(layout) as *mut StringObj;
    ptr::write(
        obj,
        StringObj {
            refcount: 1,
            len,
            data: [],
        },
    );
    if len > 0 {
        ptr::copy_nonoverlapping(data, (*obj).data.as_mut_ptr(), len);
    }
    *(*obj).data.as_mut_ptr().add(len) = 0;
    alloc_value(TAG_STRING, obj as u64)
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_closure(func: *const u8, env: *mut c_void) -> *mut Value {
    let obj = allocate(Layout::new::<ClosureObj>()) as *mut ClosureObj;
    ptr::write(
        obj,
        ClosureObj {
            refcount: 1,
            func: std::mem::transmute::<*const u8, BildaFn>(func),
            env,
        },
    );
    alloc_value(TAG_FUNCTION, obj as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_apply(closure: *mut Value, arg: *mut Value) -> *mut Value {
    let obj = (*closure).payload as *mut ClosureObj;
    ((*obj).func)(arg, (*obj).env)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_incref(value: *mut Value) {
    if value.is_null() {
        return;
    }
    (*value).refcount += 1;
    match (*value).tag {
        TAG_STRING => {
            let obj = (*value).payload as *mut StringObj;
            (*obj).refcount += 1;
        }
        TAG_MAP => {
            let obj = (*value).payload as *mut MapObj;
            (*obj).refcount += 1;
        }
        TAG_FUNCTION => {
            let obj = (*value).payload as *mut ClosureObj;
            (*obj).refcount += 1;
        }
        _ => {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_decref(value: *mut Value) {
    if value.is_null() {
        return;
    }
    (*value).refcount -= 1;
    if (*value).refcount > 0 {
        return;
    }
    match (*value).tag {
        TAG_STRING => {
            let obj = (*value).payload as *mut StringObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_string_obj(obj);
            }
        }
        TAG_MAP => {
            let obj = (*value).payload as *mut MapObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_map_obj(obj);
            }
        }
        TAG_FUNCTION => {
            let obj = (*value).payload as *mut ClosureObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_closure_obj(obj);
            }
        }
        _ => {}
    }
    dealloc(value as *mut u8, Layout::new::<Value>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_print(value: *mut Value) {
    match (*value).tag {
        TAG_INT => print!("{}", (*value).payload as isize),
        TAG_BOOL => print!(
            "{}",
            if (*value).payload != 0 {
                "True"
            } else {
                "False"
            }
        ),
        TAG_FLOAT => print!("{}", f64::from_bits((*value).payload)),
        TAG_STRING => {
            let obj = (*value).payload as *const StringObj;
            let data = slice::from_raw_parts((*obj).data.as_ptr() as *const u8, (*obj).len);
            print!("{}", String::from_utf8_lossy(data));
        }
        TAG_MAP => {
            let obj = (*value).payload as *const MapObj;
            if !(*obj).name.is_null() {
                print!(
                    "{} ",
                    CStr::from_ptr((*obj).name).to_string_lossy()
                );
            }
            print!("{{ ");
            for i in 0..(*obj).len {
                let entry = &*(*obj).entries.add(i);
                print!(
                    "{} = ",
                    CStr::from_ptr(entry.name).to_string_lossy()
                );
                bilda_print(entry.value);
                if i + 1 < (*obj).len {
                    print!(", ");
                }
            }
            print!(" }}");
        }
        TAG_FUNCTION => print!("<function>"),
        _ => print!("<unknown>"),
    }
}
