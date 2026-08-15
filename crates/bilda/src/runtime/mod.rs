#![allow(clippy::missing_safety_doc)]

use std::alloc::{Layout, alloc, dealloc};
use std::cell::RefCell;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::{ptr, slice};

use crate::runtime::value::{
    BildaFn, ClosureObj, MapEntry, MapObj, StringObj, TAG_BOOL, TAG_FLOAT, TAG_FUNCTION, TAG_INT,
    TAG_MAP, TAG_STRING, Value,
};

pub mod value;

pub use crate::runtime::value::{BildaFn as FunctionPointer, Value as RawValue};

enum Allocation {
    Bytes(*mut u8, Layout),
}

struct Arena {
    allocs: Vec<Allocation>,
}

impl Arena {
    const fn new() -> Self {
        Self { allocs: Vec::new() }
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { alloc(layout) };
        self.allocs.push(Allocation::Bytes(ptr, layout));
        ptr
    }

    fn reset(&mut self) {
        for allocation in self.allocs.drain(..) {
            let Allocation::Bytes(ptr, layout) = allocation;
            unsafe { dealloc(ptr, layout) };
        }
    }
}

thread_local! {
    static ARENA: RefCell<Arena> = const { RefCell::new(Arena::new()) };
}

fn alloc_value(tag: u64, payload: u64) -> *mut Value {
    unsafe {
        let value =
            ARENA.with(|arena| arena.borrow_mut().alloc(Layout::new::<Value>())) as *mut Value;
        ptr::write(value, Value { tag, payload });
        value
    }
}

fn copy_name(data: *const c_char, len: usize) -> *const c_char {
    unsafe {
        let layout = Layout::from_size_align_unchecked(len + 1, 1);
        let ptr = ARENA.with(|arena| arena.borrow_mut().alloc(layout)) as *mut c_char;
        if len > 0 {
            ptr::copy_nonoverlapping(data, ptr, len);
        }
        *ptr.add(len) = 0;
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_reset_arena() {
    ARENA.with(|arena| arena.borrow_mut().reset());
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
    unsafe {
        let layout = Layout::from_size_align_unchecked(
            size_of::<StringObj>() + len + 1,
            align_of::<StringObj>(),
        );
        let obj = ARENA.with(|arena| arena.borrow_mut().alloc(layout)) as *mut StringObj;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_alloc_map(name_data: *const c_char, name_len: usize) -> *mut Value {
    unsafe {
        let name = if name_len > 0 {
            copy_name(name_data, name_len)
        } else {
            ptr::null()
        };

        let obj_layout = Layout::new::<MapObj>();
        let obj = ARENA.with(|arena| arena.borrow_mut().alloc(obj_layout)) as *mut MapObj;

        let capacity = 4usize;
        let entries_layout = Layout::from_size_align_unchecked(
            capacity * size_of::<MapEntry>(),
            align_of::<MapEntry>(),
        );
        let entries = ARENA.with(|arena| arena.borrow_mut().alloc(entries_layout)) as *mut MapEntry;

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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_map_rename(
    map: *mut Value,
    name_data: *const c_char,
    name_len: usize,
) {
    unsafe {
        let obj = (*map).payload as *mut MapObj;
        (*obj).name = copy_name(name_data, name_len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_map_set(
    map: *mut Value,
    name_data: *const c_char,
    name_len: usize,
    value: *mut Value,
) {
    unsafe {
        let obj = (*map).payload as *mut MapObj;
        if (*obj).len == (*obj).capacity {
            let old_capacity = (*obj).capacity;
            let new_capacity = old_capacity * 2;
            let new_layout = Layout::from_size_align_unchecked(
                new_capacity * size_of::<MapEntry>(),
                align_of::<MapEntry>(),
            );
            let new_entries = alloc(new_layout) as *mut MapEntry;
            ARENA.with(|arena| {
                arena
                    .borrow_mut()
                    .allocs
                    .push(Allocation::Bytes(new_entries as *mut u8, new_layout))
            });
            ptr::copy_nonoverlapping((*obj).entries, new_entries, (*obj).len);
            (*obj).entries = new_entries;
            (*obj).capacity = new_capacity;
        }

        let name = copy_name(name_data, name_len);
        let entry = (*obj).entries.add((*obj).len);
        ptr::write(entry, MapEntry { name, value });
        bilda_incref(value);
        (*obj).len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_make_closure(func: *const u8, env: *mut c_void) -> *mut Value {
    unsafe {
        let obj = ARENA.with(|arena| arena.borrow_mut().alloc(Layout::new::<ClosureObj>()))
            as *mut ClosureObj;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_apply(closure: *mut Value, arg: *mut Value) -> *mut Value {
    unsafe {
        let obj = (*closure).payload as *mut ClosureObj;
        ((*obj).func)(arg, (*obj).env)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_incref(value: *mut Value) {
    unsafe {
        if value.is_null() {
            return;
        }
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_decref(value: *mut Value) {
    unsafe {
        if value.is_null() {
            return;
        }
        match (*value).tag {
            TAG_STRING => {
                let obj = (*value).payload as *mut StringObj;
                (*obj).refcount -= 1;
            }
            TAG_MAP => {
                let obj = (*value).payload as *mut MapObj;
                (*obj).refcount -= 1;
            }
            TAG_FUNCTION => {
                let obj = (*value).payload as *mut ClosureObj;
                (*obj).refcount -= 1;
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bilda_print(value: *mut Value) {
    unsafe {
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
                        std::ffi::CStr::from_ptr((*obj).name).to_string_lossy()
                    );
                }
                print!("{{ ");
                for i in 0..(*obj).len {
                    let entry = &(*(*obj).entries.add(i));
                    print!(
                        "{} = ",
                        std::ffi::CStr::from_ptr(entry.name).to_string_lossy()
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
}
