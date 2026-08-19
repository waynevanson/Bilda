#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::ffi::c_void;
use std::ptr;

use crate::runtime::alloc::allocate;
use crate::runtime::value::{BildaFn, ClosureObj, Value, TAG_FUNCTION};
use crate::runtime::value_ops::alloc_value;

pub unsafe fn free_closure_obj(obj: *mut ClosureObj) {
    if obj.is_null() {
        return;
    }
    // The environment pointer is opaque and currently always null in JIT output.
    dealloc(obj as *mut u8, Layout::new::<ClosureObj>());
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
