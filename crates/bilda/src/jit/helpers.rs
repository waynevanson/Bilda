use std::collections::HashMap;

use cranelift::codegen::ir::FuncRef;
use cranelift::prelude::{AbiParam, FunctionBuilder, Type};
use cranelift::prelude::types::F64;
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};

use crate::jit::error::CompileError;
use crate::jit::types::Types;

pub struct RuntimeHelpers {
    funcs: HashMap<&'static str, FuncId>,
}

impl RuntimeHelpers {
    pub fn declare(module: &mut JITModule, types: Types) -> Result<Self, CompileError> {
        let mut funcs = HashMap::new();

        let specs: [(&'static str, &[Type], &[Type]); 17] = [
            ("bilda_make_int", &[types.int], &[types.pointer]),
            ("bilda_make_bool", &[types.bool], &[types.pointer]),
            ("bilda_make_float", &[F64], &[types.pointer]),
            ("bilda_make_unit", &[], &[types.pointer]),
            (
                "bilda_make_string",
                &[types.pointer, types.int],
                &[types.pointer],
            ),
            (
                "bilda_concat",
                &[types.pointer, types.pointer],
                &[types.pointer],
            ),
            (
                "bilda_alloc_map",
                &[types.pointer, types.int],
                &[types.pointer],
            ),
            (
                "bilda_map_rename",
                &[types.pointer, types.pointer, types.int],
                &[],
            ),
            (
                "bilda_map_set",
                &[types.pointer, types.pointer, types.int, types.pointer],
                &[],
            ),
            ("bilda_make_list", &[], &[types.pointer]),
            (
                "bilda_list_push",
                &[types.pointer, types.pointer],
                &[],
            ),
            (
                "bilda_make_closure",
                &[types.pointer, types.pointer, types.int],
                &[types.pointer],
            ),
            (
                "bilda_capture_get",
                &[types.pointer, types.int],
                &[types.pointer],
            ),
            (
                "bilda_apply",
                &[types.pointer, types.pointer],
                &[types.pointer],
            ),
            ("bilda_incref", &[types.pointer], &[]),
            ("bilda_decref", &[types.pointer], &[]),
            ("bilda_print", &[types.pointer], &[]),
        ];

        for (name, params, returns) in specs {
            let mut sig = module.make_signature();
            for &ty in params {
                sig.params.push(AbiParam::new(ty));
            }
            for &ty in returns {
                sig.returns.push(AbiParam::new(ty));
            }
            let id = module.declare_function(name, Linkage::Import, &sig)?;
            funcs.insert(name, id);
        }

        Ok(Self { funcs })
    }

    pub fn get(&self, name: &'static str) -> FuncId {
        self.funcs[name]
    }

    pub fn declare_in_func(
        &self,
        module: &mut JITModule,
        bcx: &mut FunctionBuilder,
        name: &'static str,
    ) -> FuncRef {
        let id = self.get(name);
        module.declare_func_in_func(id, bcx.func)
    }
}
