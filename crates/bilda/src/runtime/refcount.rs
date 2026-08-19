#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};

use crate::runtime::closure::free_closure_obj;
use crate::runtime::map::free_map_obj;
use crate::runtime::string::free_string_obj;
use crate::runtime::value::{Value, TAG_FUNCTION, TAG_MAP, TAG_STRING};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_incref(value: *mut Value) {
    if value.is_null() {
        return;
    }
    (*value).refcount += 1;
    match (*value).tag {
        TAG_STRING => {
            let obj = (*value).payload as *mut crate::runtime::value::StringObj;
            (*obj).refcount += 1;
        }
        TAG_MAP => {
            let obj = (*value).payload as *mut crate::runtime::value::MapObj;
            (*obj).refcount += 1;
        }
        TAG_FUNCTION => {
            let obj = (*value).payload as *mut crate::runtime::value::ClosureObj;
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
            let obj = (*value).payload as *mut crate::runtime::value::StringObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_string_obj(obj);
            }
        }
        TAG_MAP => {
            let obj = (*value).payload as *mut crate::runtime::value::MapObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_map_obj(obj);
            }
        }
        TAG_FUNCTION => {
            let obj = (*value).payload as *mut crate::runtime::value::ClosureObj;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 {
                free_closure_obj(obj);
            }
        }
        _ => {}
    }
    dealloc(value as *mut u8, Layout::new::<Value>());
}
