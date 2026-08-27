use std::ffi::c_void;
use std::os::raw::c_char;

pub const TAG_INT: u64 = 0;
pub const TAG_BOOL: u64 = 1;
pub const TAG_STRING: u64 = 2;
pub const TAG_MAP: u64 = 3;
pub const TAG_FUNCTION: u64 = 4;
pub const TAG_FLOAT: u64 = 5;
pub const TAG_LIST: u64 = 6;
pub const TAG_UNIT: u64 = 7;

/// A Bilda value is a reference-counted pair of a tag and a payload.
#[repr(C)]
pub struct Value {
    pub refcount: usize,
    pub tag: u64,
    pub payload: u64,
}

#[repr(C)]
pub struct StringObj {
    pub refcount: usize,
    pub len: usize,
    pub data: [c_char; 0],
}

#[repr(C)]
pub struct MapEntry {
    pub name: *const c_char,
    pub value: *mut Value,
}

#[repr(C)]
pub struct MapObj {
    pub refcount: usize,
    pub name: *const c_char,
    pub len: usize,
    pub capacity: usize,
    pub entries: *mut MapEntry,
}

#[repr(C)]
pub struct ListObj {
    pub refcount: usize,
    pub len: usize,
    pub capacity: usize,
    pub items: *mut *mut Value,
}

#[repr(C)]
pub struct CaptureObj {
    pub refcount: usize,
    pub len: usize,
    pub capacity: usize,
    pub values: *mut *mut Value,
}

pub type BildaFn = extern "C" fn(*mut Value, *mut c_void) -> *mut Value;

#[repr(C)]
pub struct ClosureObj {
    pub refcount: usize,
    pub func: BildaFn,
    pub env: *mut c_void,
    pub remaining: usize,
}
