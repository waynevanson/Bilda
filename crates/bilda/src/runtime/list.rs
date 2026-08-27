#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::ptr;

use crate::runtime::alloc::allocate;
use crate::runtime::refcount::bilda_incref;
use crate::runtime::value::{ListObj, Value, TAG_LIST};
use crate::runtime::value_ops::alloc_value;

pub unsafe fn free_list_obj(obj: *mut ListObj) {
    if obj.is_null() {
        return;
    }
    for i in 0..(*obj).len {
        crate::runtime::refcount::bilda_decref(*(*obj).items.add(i));
    }
    if !(*obj).items.is_null() && (*obj).capacity > 0 {
        let layout = Layout::array::<*mut Value>((*obj).capacity).unwrap_unchecked();
        dealloc((*obj).items as *mut u8, layout);
    }
    dealloc(obj as *mut u8, Layout::new::<ListObj>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_list() -> *mut Value {
    let obj = allocate(Layout::new::<ListObj>()) as *mut ListObj;

    let capacity = 4usize;
    let items_layout = Layout::array::<*mut Value>(capacity).unwrap_unchecked();
    let items = allocate(items_layout) as *mut *mut Value;

    ptr::write(
        obj,
        ListObj {
            refcount: 1,
            len: 0,
            capacity,
            items,
        },
    );

    alloc_value(TAG_LIST, obj as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_list_push(list: *mut Value, value: *mut Value) {
    let obj = (*list).payload as *mut ListObj;
    if (*obj).len == (*obj).capacity {
        let old_capacity = (*obj).capacity;
        let new_capacity = old_capacity * 2;
        let new_layout = Layout::array::<*mut Value>(new_capacity).unwrap_unchecked();
        let new_items = allocate(new_layout) as *mut *mut Value;
        ptr::copy_nonoverlapping((*obj).items, new_items, (*obj).len);
        let old_layout = Layout::array::<*mut Value>(old_capacity).unwrap_unchecked();
        dealloc((*obj).items as *mut u8, old_layout);
        (*obj).items = new_items;
        (*obj).capacity = new_capacity;
    }

    *(*obj).items.add((*obj).len) = value;
    bilda_incref(value);
    (*obj).len += 1;
}
