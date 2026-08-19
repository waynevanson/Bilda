use std::collections::HashMap;

use cranelift::prelude::{AbiParam, Type};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::{Ast, Expression};
use crate::jit::collect::collect_lambdas_ast;
use crate::jit::error::CompileError;

pub struct Lambdas<'a> {
    pub expressions: Vec<*const Expression<'a>>,
    pub func_ids: HashMap<*const Expression<'a>, FuncId>,
}

impl<'a> Lambdas<'a> {
    pub fn collect(ast: &Ast<'a>) -> Self {
        let mut expressions = Vec::new();
        collect_lambdas_ast(ast, &mut expressions);
        Self {
            expressions,
            func_ids: HashMap::new(),
        }
    }

    pub fn declare(
        mut self,
        module: &mut JITModule,
        pointer_type: Type,
    ) -> Result<Self, CompileError> {
        for (idx, &lambda_expr) in self.expressions.iter().enumerate() {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(pointer_type));
            sig.params.push(AbiParam::new(pointer_type));
            sig.returns.push(AbiParam::new(pointer_type));
            let name = format!("lambda_{idx}");
            let id = module.declare_function(&name, Linkage::Local, &sig)?;
            self.func_ids.insert(lambda_expr, id);
        }
        Ok(self)
    }
}

pub struct LambdaTable<'a> {
    pub func_ids: HashMap<*const Expression<'a>, FuncId>,
}

impl<'a> LambdaTable<'a> {
    pub fn empty() -> Self {
        Self {
            func_ids: HashMap::new(),
        }
    }

    pub fn get(&self, expr: *const Expression<'a>) -> Option<FuncId> {
        self.func_ids.get(&expr).copied()
    }
}
