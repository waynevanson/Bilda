#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::ffi::c_char;
use std::ptr;

use crate::runtime::alloc::allocate;
use crate::runtime::value::{StringObj, Value, TAG_STRING};
use crate::runtime::value_ops::alloc_value;

pub unsafe fn string_layout(len: usize) -> Layout {
    let size = std::mem::size_of::<StringObj>() + len + 1;
    let align = std::mem::align_of::<StringObj>();
    let padded = (size + align - 1) & !(align - 1);
    Layout::from_size_align_unchecked(padded, align)
}

pub unsafe fn free_string_obj(obj: *mut StringObj) {
    let layout = string_layout((*obj).len);
    dealloc(obj as *mut u8, layout);
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
pub unsafe extern "C" fn bilda_concat(a: *mut Value, b: *mut Value) -> *mut Value {
    let sa = (*a).payload as *const StringObj;
    let sb = (*b).payload as *const StringObj;
    let len = (*sa).len + (*sb).len;

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
    if (*sa).len > 0 {
        ptr::copy_nonoverlapping((*sa).data.as_ptr(), (*obj).data.as_mut_ptr(), (*sa).len);
    }
    if (*sb).len > 0 {
        ptr::copy_nonoverlapping(
            (*sb).data.as_ptr(),
            (*obj).data.as_mut_ptr().add((*sa).len),
            (*sb).len,
        );
    }
    *(*obj).data.as_mut_ptr().add(len) = 0;
    alloc_value(TAG_STRING, obj as u64)
}
