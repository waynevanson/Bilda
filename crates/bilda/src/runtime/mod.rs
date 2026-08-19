pub mod value;

pub mod alloc;
pub mod closure;
pub mod map;
pub mod print;
pub mod refcount;
pub mod string;
pub mod value_ops;

pub use crate::runtime::value::Value as RawValue;
pub use crate::runtime::closure::{bilda_apply, bilda_make_closure};
pub use crate::runtime::map::{bilda_alloc_map, bilda_map_rename, bilda_map_set};
pub use crate::runtime::refcount::{bilda_decref, bilda_incref};
pub use crate::runtime::string::bilda_make_string;
pub use crate::runtime::value_ops::{bilda_make_bool, bilda_make_float, bilda_make_int};
pub use crate::runtime::print::bilda_print;
