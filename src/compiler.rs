use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::ast::{Ast, Block, Expr, InfixOp};

pub struct Compiler {
    module: JITModule,
    builder_context: FunctionBuilderContext,
}

impl Compiler {
    pub fn new() -> Result<Self, String> {
        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder
            .finish(settings::Flags::new(settings::builder()))
            .map_err(|e| e.to_string())?;
        let module =
            JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names()));
        Ok(Self {
            module,
            builder_context: FunctionBuilderContext::new(),
        })
    }

    pub fn compile(&mut self, ast: &Ast) -> Result<fn() -> i64, String> {
        let mut ctx = self.module.make_context();
        ctx.func.signature.returns.push(AbiParam::new(types::I64));

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);
        let entry = builder.create_block();
        builder.switch_to_block(entry);
        builder.append_block_params_for_function_params(entry);
        builder.seal_block(entry);

        let mut env: HashMap<String, Value> = HashMap::new();
        let mut result = builder.ins().iconst(types::I64, 0);

        for block in &ast.0 {
            match block {
                Block::Let(let_block) => {
                    for binding in &let_block.bindings {
                        let value = compile_expr(&binding.value, &mut builder, &env)?;
                        env.insert(binding.name.clone(), value);
                    }
                    result = compile_expr(&let_block.body, &mut builder, &env)?;
                }
                _ => return Err("only Let blocks supported by JIT".to_string()),
            }
        }

        builder.ins().return_(&[result]);
        builder.finalize();

        let id = self
            .module
            .declare_function("main", Linkage::Local, &ctx.func.signature)
            .map_err(|e| e.to_string())?;
        self.module
            .define_function(id, &mut ctx)
            .map_err(|e| e.to_string())?;
        self.module
            .finalize_definitions()
            .map_err(|e| e.to_string())?;

        let code = self.module.get_finalized_function(id);
        Ok(unsafe { std::mem::transmute::<*const u8, fn() -> i64>(code) })
    }
}

fn compile_expr(
    expr: &Expr,
    builder: &mut FunctionBuilder,
    env: &HashMap<String, Value>,
) -> Result<Value, String> {
    match expr {
        Expr::Number(text) => {
            let n = text
                .parse::<i64>()
                .map_err(|_| format!("invalid number: {text}"))?;
            Ok(builder.ins().iconst(types::I64, n))
        }
        Expr::Ident(name) => env
            .get(name)
            .copied()
            .ok_or_else(|| format!("undefined identifier: {name}")),
        Expr::Infix { op, left, right } => {
            let left = compile_expr(left, builder, env)?;
            let right = compile_expr(right, builder, env)?;
            let value = match op {
                InfixOp::Add => builder.ins().iadd(left, right),
                InfixOp::Sub => builder.ins().isub(left, right),
                InfixOp::Mul => builder.ins().imul(left, right),
            };
            Ok(value)
        }
        _ => Err("unsupported expression in JIT".to_string()),
    }
}
