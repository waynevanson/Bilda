#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::ffi::c_void;
use std::ptr;

use crate::runtime::alloc::allocate;
use crate::runtime::refcount::{bilda_decref, bilda_incref};
use crate::runtime::value::{BildaFn, CaptureObj, ClosureObj, Value, TAG_FUNCTION};
use crate::runtime::value_ops::alloc_value;

pub unsafe fn free_capture_obj(env: *mut CaptureObj) {
    if env.is_null() {
        return;
    }
    for i in 0..(*env).len {
        bilda_decref(*(*env).values.add(i));
    }
    if !(*env).values.is_null() && (*env).capacity > 0 {
        let layout = Layout::array::<*mut Value>((*env).capacity).unwrap_unchecked();
        dealloc((*env).values as *mut u8, layout);
    }
    dealloc(env as *mut u8, Layout::new::<CaptureObj>());
}

pub unsafe fn free_closure_obj(obj: *mut ClosureObj) {
    if obj.is_null() {
        return;
    }
    free_capture_obj((*obj).env as *mut CaptureObj);
    dealloc(obj as *mut u8, Layout::new::<ClosureObj>());
}

pub unsafe fn capture_push(env: *mut CaptureObj, value: *mut Value) -> *mut CaptureObj {
    let new_env = allocate(Layout::new::<CaptureObj>()) as *mut CaptureObj;
    let capacity = 4usize.max(if env.is_null() { 0 } else { (*env).len + 1 });
    let values_layout = Layout::array::<*mut Value>(capacity).unwrap_unchecked();
    let values = allocate(values_layout) as *mut *mut Value;

    ptr::write(
        new_env,
        CaptureObj {
            refcount: 1,
            len: 0,
            capacity,
            values,
        },
    );

    if !env.is_null() {
        for i in 0..(*env).len {
            let v = *(*env).values.add(i);
            *(*new_env).values.add((*new_env).len) = v;
            bilda_incref(v);
            (*new_env).len += 1;
        }
    }

    *(*new_env).values.add((*new_env).len) = value;
    bilda_incref(value);
    (*new_env).len += 1;

    new_env
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_capture_get(env: *mut CaptureObj, idx: usize) -> *mut Value {
    *(*env).values.add(idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_closure(
    func: *const u8,
    env: *mut c_void,
    remaining: usize,
) -> *mut Value {
    let obj = allocate(Layout::new::<ClosureObj>()) as *mut ClosureObj;
    ptr::write(
        obj,
        ClosureObj {
            refcount: 1,
            func: std::mem::transmute::<*const u8, BildaFn>(func),
            env,
            remaining,
        },
    );
    alloc_value(TAG_FUNCTION, obj as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_apply(closure: *mut Value, arg: *mut Value) -> *mut Value {
    let obj = (*closure).payload as *mut ClosureObj;

    if (*obj).remaining > 1 {
        let env = capture_push((*obj).env as *mut CaptureObj, arg);
        bilda_decref(arg);
        return bilda_make_closure((*obj).func as *const u8, env as *mut c_void, (*obj).remaining - 1);
    }

    ((*obj).func)(arg, (*obj).env)
}
