use cranelift_jit::JITBuilder;

use crate::runtime::{
    bilda_alloc_map, bilda_apply, bilda_decref, bilda_incref, bilda_make_bool,
    bilda_make_closure, bilda_make_float, bilda_make_int, bilda_make_string, bilda_map_rename,
    bilda_map_set, bilda_print,
};

pub fn register_runtime_symbols(builder: &mut JITBuilder) {
    builder.symbol("bilda_make_int", bilda_make_int as *const u8);
    builder.symbol("bilda_make_bool", bilda_make_bool as *const u8);
    builder.symbol("bilda_make_float", bilda_make_float as *const u8);
    builder.symbol("bilda_make_string", bilda_make_string as *const u8);
    builder.symbol("bilda_alloc_map", bilda_alloc_map as *const u8);
    builder.symbol("bilda_map_rename", bilda_map_rename as *const u8);
    builder.symbol("bilda_map_set", bilda_map_set as *const u8);
    builder.symbol("bilda_make_closure", bilda_make_closure as *const u8);
    builder.symbol("bilda_apply", bilda_apply as *const u8);
    builder.symbol("bilda_incref", bilda_incref as *const u8);
    builder.symbol("bilda_decref", bilda_decref as *const u8);
    builder.symbol("bilda_print", bilda_print as *const u8);
}
