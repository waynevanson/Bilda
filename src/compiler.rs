use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::{Ast, Block, Expr, InfixOp};

pub struct Compiler {
    module: JITModule,
    builder_context: FunctionBuilderContext,
}

enum ModuleBinding {
    TypeConstructor,
    Function(FuncId),
}

impl Compiler {
    pub fn new() -> Result<Self, String> {
        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder
            .finish(settings::Flags::new(settings::builder()))
            .map_err(|e| e.to_string())?;

        extern "C" fn bilda_echo(value: i64) -> i64 {
            println!("{value}");
            0
        }

        let mut builder =
            JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbol("echo", bilda_echo as *const u8);
        let module = JITModule::new(builder);

        Ok(Self {
            module,
            builder_context: FunctionBuilderContext::new(),
        })
    }

    pub fn compile(&mut self, ast: &Ast) -> Result<fn() -> (i64, i64), String> {
        let mut module_env: HashMap<String, ModuleBinding> = HashMap::new();

        for block in &ast.0 {
            match block {
                Block::Use(use_block) => {
                    for import in &use_block.imports {
                        for name in &import.names {
                            if import.module == "std" && name == "echo" {
                                let mut sig = self.module.make_signature();
                                sig.params.push(AbiParam::new(types::I64));
                                sig.returns.push(AbiParam::new(types::I64));
                                let id = self
                                    .module
                                    .declare_function(name, Linkage::Import, &sig)
                                    .map_err(|e| e.to_string())?;
                                module_env.insert(name.clone(), ModuleBinding::Function(id));
                            } else {
                                return Err(format!(
                                    "unsupported import {name} from {}",
                                    import.module
                                ));
                            }
                        }
                    }
                }
                Block::Type(type_block) => {
                    for binding in &type_block.bindings {
                        module_env.insert(binding.name.clone(), ModuleBinding::TypeConstructor);
                    }
                }
                _ => {}
            }
        }

        let mut ctx = self.module.make_context();
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        ctx.func.signature.returns.push(AbiParam::new(types::I64));

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);
        let entry = builder.create_block();
        builder.switch_to_block(entry);
        builder.append_block_params_for_function_params(entry);
        builder.seal_block(entry);

        let mut local_env: HashMap<String, Value> = HashMap::new();
        let mut result_value = builder.ins().iconst(types::I64, 0);
        let mut result_is_number = false;

        for block in &ast.0 {
            match block {
                Block::Use(_) | Block::Type(_) => {}
                Block::Let(let_block) => {
                    for binding in &let_block.bindings {
                        let (value, _) = compile_expr(
                            &binding.value,
                            &mut builder,
                            &mut self.module,
                            &local_env,
                            &module_env,
                        )?;
                        local_env.insert(binding.name.clone(), value);
                    }
                    let (value, is_number) = compile_expr(
                        &let_block.body,
                        &mut builder,
                        &mut self.module,
                        &local_env,
                        &module_env,
                    )?;
                    result_value = value;
                    result_is_number = is_number;
                }
            }
        }

        let tag = builder
            .ins()
            .iconst(types::I64, if result_is_number { 1 } else { 0 });
        builder.ins().return_(&[result_value, tag]);
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
        Ok(unsafe { std::mem::transmute::<*const u8, fn() -> (i64, i64)>(code) })
    }
}

fn compile_expr(
    expr: &Expr,
    builder: &mut FunctionBuilder,
    module: &mut JITModule,
    local_env: &HashMap<String, Value>,
    module_env: &HashMap<String, ModuleBinding>,
) -> Result<(Value, bool), String> {
    match expr {
        Expr::Number(text) => {
            let n = text
                .parse::<i64>()
                .map_err(|_| format!("invalid number: {text}"))?;
            Ok((builder.ins().iconst(types::I64, n), true))
        }
        Expr::Ident(name) => {
            if let Some(binding) = module_env.get(name) {
                match binding {
                    ModuleBinding::TypeConstructor => Err(format!(
                        "type constructor {name} must be applied to an argument"
                    )),
                    ModuleBinding::Function(_) => {
                        Err(format!("function {name} must be applied to an argument"))
                    }
                }
            } else {
                local_env
                    .get(name)
                    .copied()
                    .map(|v| (v, true))
                    .ok_or_else(|| format!("undefined identifier: {name}"))
            }
        }
        Expr::Apply { func, arg } => {
            if let Expr::Ident(name) = func.as_ref() {
                match module_env.get(name) {
                    Some(ModuleBinding::TypeConstructor) => {
                        compile_expr(arg, builder, module, local_env, module_env)
                    }
                    Some(ModuleBinding::Function(func_id)) => {
                        let (arg_value, _) =
                            compile_expr(arg, builder, module, local_env, module_env)?;
                        let func_ref =
                            module.declare_func_in_func(*func_id, builder.func);
                        let call = builder.ins().call(func_ref, &[arg_value]);
                        let value = builder.inst_results(call)[0];
                        Ok((value, false))
                    }
                    None => Err(format!("unsupported function: {name}")),
                }
            } else {
                Err("unsupported application".to_string())
            }
        }
        Expr::Infix { op, left, right } => {
            let (left, _) = compile_expr(left, builder, module, local_env, module_env)?;
            let (right, _) = compile_expr(right, builder, module, local_env, module_env)?;
            let value = match op {
                InfixOp::Add => builder.ins().iadd(left, right),
                InfixOp::Sub => builder.ins().isub(left, right),
                InfixOp::Mul => builder.ins().imul(left, right),
            };
            Ok((value, true))
        }
        _ => Err("unsupported expression in JIT".to_string()),
    }
}
