#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::CStr;
use std::slice;

use crate::runtime::value::{
    MapObj, StringObj, Value, TAG_BOOL, TAG_FLOAT, TAG_FUNCTION, TAG_INT, TAG_MAP, TAG_STRING,
};

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
                print!("{} ", CStr::from_ptr((*obj).name).to_string_lossy());
            }
            print!("{{ ");
            for i in 0..(*obj).len {
                let entry = &*(*obj).entries.add(i);
                print!("{} = ", CStr::from_ptr(entry.name).to_string_lossy());
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
