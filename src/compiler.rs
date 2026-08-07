use std::collections::{HashMap, HashSet};

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::{Ast, Block, Expr, InfixOp};

// -----------------------------------------------------------------------------
// Runtime helpers called from JIT-compiled code.
// -----------------------------------------------------------------------------

static HEAP: std::sync::Mutex<Vec<Box<[u8]>>> = std::sync::Mutex::new(Vec::new());

#[repr(C)]
struct ValuePair {
    data: i64,
    tag: i64,
}

extern "C" fn bilda_alloc(size: i64) -> i64 {
    let mut heap = HEAP.lock().unwrap();
    let mut buf = vec![0u8; size as usize].into_boxed_slice();
    let ptr = buf.as_mut_ptr();
    heap.push(buf);
    ptr as i64
}

extern "C" fn bilda_echo(data: i64, tag: i64) -> ValuePair {
    if tag == 1 {
        println!("{data}");
    } else if tag == 2 {
        println!("<closure>");
    }
    ValuePair { data: 0, tag: 0 }
}

// -----------------------------------------------------------------------------
// JIT compiler.
// -----------------------------------------------------------------------------

pub struct Compiler {
    module: JITModule,
    alloc_id: FuncId,
    lambda_counter: usize,
}

enum ModuleBinding {
    TypeConstructor,
    Function(FuncId),
}

/// Tag values used at runtime.
const TAG_UNIT: i64 = 0;
const TAG_NUMBER: i64 = 1;
const TAG_CLOSURE: i64 = 2;

/// In-memory layout of a closure object.
struct ClosureLayout;

impl ClosureLayout {
    const CODE_OFFSET: i32 = 0;
    const ARITY_OFFSET: i32 = 8;
    const FREE_LEN_OFFSET: i32 = 16;
    const APPLIED_LEN_OFFSET: i32 = 24;
    const DATA_OFFSET: i32 = 32;
    const VALUE_SIZE: i32 = 16;

    fn size(free_len: i64, applied_len: i64) -> i64 {
        Self::DATA_OFFSET as i64 + (free_len + applied_len) * Self::VALUE_SIZE as i64
    }

    fn free_data_offset(index: i64) -> i32 {
        Self::DATA_OFFSET + (index * Self::VALUE_SIZE as i64) as i32
    }

    fn applied_data_offset(free_len: i64, index: i64) -> i32 {
        Self::DATA_OFFSET + ((free_len + index) * Self::VALUE_SIZE as i64) as i32
    }
}

/// A compiled value is a `(data, tag)` pair.
type CompiledValue = (Value, Value);

impl Compiler {
    pub fn new() -> Result<Self, String> {
        let isa_builder = cranelift_native::builder().map_err(|e| e.to_string())?;
        let isa = isa_builder
            .finish(settings::Flags::new(settings::builder()))
            .map_err(|e| e.to_string())?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbol("alloc", bilda_alloc as *const u8);
        builder.symbol("echo", bilda_echo as *const u8);
        let mut module = JITModule::new(builder);

        let mut alloc_sig = module.make_signature();
        alloc_sig.params.push(AbiParam::new(types::I64));
        alloc_sig.returns.push(AbiParam::new(types::I64));
        let alloc_id = module
            .declare_function("alloc", Linkage::Import, &alloc_sig)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            module,
            alloc_id,
            lambda_counter: 0,
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
                                sig.params.push(AbiParam::new(types::I64));
                                sig.returns.push(AbiParam::new(types::I64));
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

        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);
        let entry = builder.create_block();
        builder.switch_to_block(entry);
        builder.append_block_params_for_function_params(entry);
        builder.seal_block(entry);

        let mut local_env: HashMap<String, CompiledValue> = HashMap::new();
        let mut result = unit_const(&mut builder);

        for block in &ast.0 {
            match block {
                Block::Use(_) | Block::Type(_) => {}
                Block::Let(let_block) => {
                    for binding in &let_block.bindings {
                        let value = compile_expr(
                            &binding.value,
                            self,
                            &mut builder,
                            &local_env,
                            &module_env,
                        )?;
                        local_env.insert(binding.name.clone(), value);
                    }
                    result =
                        compile_expr(&let_block.body, self, &mut builder, &local_env, &module_env)?;
                }
            }
        }

        let (result_data, result_tag) = result;
        builder.ins().return_(&[result_data, result_tag]);
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

fn unit_const(builder: &mut FunctionBuilder) -> CompiledValue {
    let data = builder.ins().iconst(types::I64, 0);
    let tag = builder.ins().iconst(types::I64, TAG_UNIT);
    (data, tag)
}

fn number_const(builder: &mut FunctionBuilder, n: i64) -> CompiledValue {
    let data = builder.ins().iconst(types::I64, n);
    let tag = builder.ins().iconst(types::I64, TAG_NUMBER);
    (data, tag)
}

fn closure_const(builder: &mut FunctionBuilder, ptr: Value) -> CompiledValue {
    let tag = builder.ins().iconst(types::I64, TAG_CLOSURE);
    (ptr, tag)
}

fn compile_expr(
    expr: &Expr,
    compiler: &mut Compiler,
    builder: &mut FunctionBuilder,
    local_env: &HashMap<String, CompiledValue>,
    module_env: &HashMap<String, ModuleBinding>,
) -> Result<CompiledValue, String> {
    match expr {
        Expr::Number(text) => {
            let n = text
                .parse::<i64>()
                .map_err(|_| format!("invalid number: {text}"))?;
            Ok(number_const(builder, n))
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
                    .ok_or_else(|| format!("undefined identifier: {name}"))
            }
        }
        Expr::Apply { func, arg } => {
            if let Expr::Ident(name) = func.as_ref() {
                match module_env.get(name) {
                    Some(ModuleBinding::TypeConstructor) => {
                        compile_expr(arg, compiler, builder, local_env, module_env)
                    }
                    Some(ModuleBinding::Function(func_id)) => {
                        let (arg_data, arg_tag) =
                            compile_expr(arg, compiler, builder, local_env, module_env)?;
                        let func_ref = compiler.module.declare_func_in_func(*func_id, builder.func);
                        let call = builder.ins().call(func_ref, &[arg_data, arg_tag]);
                        let results = builder.inst_results(call);
                        Ok((results[0], results[1]))
                    }
                    None => {
                        // Not a module function; compile as closure application.
                        let (func_data, func_tag) =
                            compile_expr(func, compiler, builder, local_env, module_env)?;
                        apply_closure(
                            compiler, builder, func_data, func_tag, arg, local_env, module_env,
                        )
                    }
                }
            } else {
                // General application of an arbitrary expression.
                let (func_data, func_tag) =
                    compile_expr(func, compiler, builder, local_env, module_env)?;
                apply_closure(
                    compiler, builder, func_data, func_tag, arg, local_env, module_env,
                )
            }
        }
        Expr::Infix { op, left, right } => {
            let (left_data, _) = compile_expr(left, compiler, builder, local_env, module_env)?;
            let (right_data, _) = compile_expr(right, compiler, builder, local_env, module_env)?;
            let value = match op {
                InfixOp::Add => builder.ins().iadd(left_data, right_data),
                InfixOp::Sub => builder.ins().isub(left_data, right_data),
                InfixOp::Mul => builder.ins().imul(left_data, right_data),
            };
            let tag = builder.ins().iconst(types::I64, TAG_NUMBER);
            Ok((value, tag))
        }
        Expr::Lambda { params, body } => {
            compile_lambda(compiler, builder, params, body, local_env, module_env)
        }
        _ => Err("unsupported expression in JIT".to_string()),
    }
}

fn apply_closure(
    compiler: &mut Compiler,
    builder: &mut FunctionBuilder,
    func_data: Value,
    _func_tag: Value,
    arg: &Expr,
    local_env: &HashMap<String, CompiledValue>,
    module_env: &HashMap<String, ModuleBinding>,
) -> Result<CompiledValue, String> {
    // For now we assume the applied value is a closure. We do not emit a runtime tag check.
    let closure_ptr = func_data;
    let (arg_data, arg_tag) = compile_expr(arg, compiler, builder, local_env, module_env)?;

    let code_ptr = builder.ins().load(
        types::I64,
        MemFlags::new(),
        closure_ptr,
        ClosureLayout::CODE_OFFSET,
    );

    let mut sig = compiler.module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let sig_ref = builder.import_signature(sig);

    let call = builder
        .ins()
        .call_indirect(sig_ref, code_ptr, &[arg_data, arg_tag, closure_ptr]);
    let results = builder.inst_results(call);
    Ok((results[0], results[1]))
}

fn compile_lambda(
    compiler: &mut Compiler,
    builder: &mut FunctionBuilder,
    params: &[String],
    body: &Expr,
    local_env: &HashMap<String, CompiledValue>,
    module_env: &HashMap<String, ModuleBinding>,
) -> Result<CompiledValue, String> {
    if params.is_empty() {
        return Err("lambda must have at least one parameter".to_string());
    }

    let module_names: HashSet<String> = module_env.keys().cloned().collect();
    let param_set: HashSet<String> = params.iter().cloned().collect();
    let free_vars: Vec<String> = free_variables(body, &module_names, &param_set)
        .into_iter()
        .collect();

    let n = params.len();
    let lambda_id = compiler.lambda_counter;
    compiler.lambda_counter += 1;

    // Declare all N function signatures up front so every thunk can reference the next.
    let mut func_ids = Vec::with_capacity(n);
    let mut sig = compiler.module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));

    for k in 1..=n {
        let name = format!("lambda_{lambda_id}_{k}");
        let id = compiler
            .module
            .declare_function(&name, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        func_ids.push(id);
    }

    // Define each function.
    for (k_idx, &func_id) in func_ids.iter().enumerate() {
        let k = k_idx + 1; // 1-based index
        let mut ctx = compiler.module.make_context();
        ctx.func.signature = sig.clone();

        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);
        let entry = builder.create_block();
        builder.switch_to_block(entry);
        builder.append_block_params_for_function_params(entry);
        builder.seal_block(entry);

        let arg_data = builder.block_params(entry)[0];
        let arg_tag = builder.block_params(entry)[1];
        let closure_ptr = builder.block_params(entry)[2];

        if k < n {
            // Thunk: accumulate the argument and return a closure for the next stage.
            let next_func_id = func_ids[k]; // k is 1-based, k < n, so k is a valid index for func_ids[k]
            let new_closure = make_next_closure(
                compiler,
                &mut builder,
                NextClosureArgs {
                    next_func_id,
                    closure_ptr,
                    new_arg: (arg_data, arg_tag),
                    new_arity: (n - k) as i64,
                    free_vars: &free_vars,
                    incoming_applied_len: k as i64 - 1,
                },
            );
            let (ptr, _) = new_closure;
            let closure_tag = builder.ins().iconst(types::I64, TAG_CLOSURE);
            builder.ins().return_(&[ptr, closure_tag]);
        } else {
            // Final stage: evaluate the body.
            let mut body_env: HashMap<String, CompiledValue> = HashMap::new();

            // Load free variables.
            for (i, name) in free_vars.iter().enumerate() {
                let offset = ClosureLayout::free_data_offset(i as i64);
                let data = builder
                    .ins()
                    .load(types::I64, MemFlags::new(), closure_ptr, offset);
                let tag = builder
                    .ins()
                    .load(types::I64, MemFlags::new(), closure_ptr, offset + 8);
                body_env.insert(name.clone(), (data, tag));
            }

            // Load previously applied arguments as the first N-1 parameters.
            for (i, name) in params.iter().enumerate().take(n - 1) {
                let offset = ClosureLayout::applied_data_offset(free_vars.len() as i64, i as i64);
                let data = builder
                    .ins()
                    .load(types::I64, MemFlags::new(), closure_ptr, offset);
                let tag = builder
                    .ins()
                    .load(types::I64, MemFlags::new(), closure_ptr, offset + 8);
                body_env.insert(name.clone(), (data, tag));
            }

            // The final parameter is the argument passed to this function.
            body_env.insert(params[n - 1].clone(), (arg_data, arg_tag));

            let result = compile_expr(body, compiler, &mut builder, &body_env, module_env)?;
            let (result_data, result_tag) = result;
            builder.ins().return_(&[result_data, result_tag]);
        }

        builder.finalize();
        compiler
            .module
            .define_function(func_id, &mut ctx)
            .map_err(|e| e.to_string())?;
    }

    // Allocate the initial closure at the creation site.
    let initial_func_id = func_ids[0];
    let code_ptr = compiler
        .module
        .declare_func_in_func(initial_func_id, builder.func);
    let code_ptr_value = builder.ins().func_addr(types::I64, code_ptr);

    let size = ClosureLayout::size(free_vars.len() as i64, 0);
    let size_value = builder.ins().iconst(types::I64, size);

    let alloc_func_ref = compiler
        .module
        .declare_func_in_func(compiler.alloc_id, builder.func);
    let alloc_call = builder.ins().call(alloc_func_ref, &[size_value]);
    let closure_ptr = builder.inst_results(alloc_call)[0];

    // Initialize header.
    builder.ins().store(
        MemFlags::new(),
        code_ptr_value,
        closure_ptr,
        ClosureLayout::CODE_OFFSET,
    );
    let arity_value = builder.ins().iconst(types::I64, n as i64);
    builder.ins().store(
        MemFlags::new(),
        arity_value,
        closure_ptr,
        ClosureLayout::ARITY_OFFSET,
    );
    let free_len_value = builder.ins().iconst(types::I64, free_vars.len() as i64);
    builder.ins().store(
        MemFlags::new(),
        free_len_value,
        closure_ptr,
        ClosureLayout::FREE_LEN_OFFSET,
    );
    let applied_len_value = builder.ins().iconst(types::I64, 0);
    builder.ins().store(
        MemFlags::new(),
        applied_len_value,
        closure_ptr,
        ClosureLayout::APPLIED_LEN_OFFSET,
    );

    // Copy free variables into the closure environment.
    for (i, name) in free_vars.iter().enumerate() {
        let (data, tag) = local_env
            .get(name)
            .copied()
            .ok_or_else(|| format!("free variable {name} not found in local environment"))?;
        let offset = ClosureLayout::free_data_offset(i as i64);
        builder
            .ins()
            .store(MemFlags::new(), data, closure_ptr, offset);
        builder
            .ins()
            .store(MemFlags::new(), tag, closure_ptr, offset + 8);
    }

    Ok(closure_const(builder, closure_ptr))
}

struct NextClosureArgs<'a> {
    next_func_id: FuncId,
    closure_ptr: Value,
    new_arg: CompiledValue,
    new_arity: i64,
    free_vars: &'a [String],
    incoming_applied_len: i64,
}

/// Build a closure for stage `k + 1` inside the thunk for stage `k`.
fn make_next_closure(
    compiler: &mut Compiler,
    builder: &mut FunctionBuilder,
    args: NextClosureArgs<'_>,
) -> CompiledValue {
    let free_len = args.free_vars.len() as i64;

    // Allocate new closure.
    let new_size = ClosureLayout::size(free_len, args.incoming_applied_len + 1);
    let size_value = builder.ins().iconst(types::I64, new_size);

    let alloc_func_ref = compiler
        .module
        .declare_func_in_func(compiler.alloc_id, builder.func);
    let alloc_call = builder.ins().call(alloc_func_ref, &[size_value]);
    let new_closure_ptr = builder.inst_results(alloc_call)[0];

    // Write header.
    let next_func_ref = compiler
        .module
        .declare_func_in_func(args.next_func_id, builder.func);
    let next_code_ptr = builder.ins().func_addr(types::I64, next_func_ref);
    builder.ins().store(
        MemFlags::new(),
        next_code_ptr,
        new_closure_ptr,
        ClosureLayout::CODE_OFFSET,
    );
    let arity_value = builder.ins().iconst(types::I64, args.new_arity);
    builder.ins().store(
        MemFlags::new(),
        arity_value,
        new_closure_ptr,
        ClosureLayout::ARITY_OFFSET,
    );
    let free_len_value = builder.ins().iconst(types::I64, free_len);
    builder.ins().store(
        MemFlags::new(),
        free_len_value,
        new_closure_ptr,
        ClosureLayout::FREE_LEN_OFFSET,
    );
    let applied_len_value = builder
        .ins()
        .iconst(types::I64, args.incoming_applied_len + 1);
    builder.ins().store(
        MemFlags::new(),
        applied_len_value,
        new_closure_ptr,
        ClosureLayout::APPLIED_LEN_OFFSET,
    );

    // Copy free variables.
    for i in 0..free_len {
        let offset = ClosureLayout::free_data_offset(i);
        let data = builder
            .ins()
            .load(types::I64, MemFlags::new(), args.closure_ptr, offset);
        let tag = builder
            .ins()
            .load(types::I64, MemFlags::new(), args.closure_ptr, offset + 8);
        builder
            .ins()
            .store(MemFlags::new(), data, new_closure_ptr, offset);
        builder
            .ins()
            .store(MemFlags::new(), tag, new_closure_ptr, offset + 8);
    }

    // Copy previously applied arguments.
    for i in 0..args.incoming_applied_len {
        let old_offset = ClosureLayout::applied_data_offset(free_len, i);
        let new_offset = ClosureLayout::applied_data_offset(free_len, i);
        let data = builder
            .ins()
            .load(types::I64, MemFlags::new(), args.closure_ptr, old_offset);
        let tag = builder.ins().load(
            types::I64,
            MemFlags::new(),
            args.closure_ptr,
            old_offset + 8,
        );
        builder
            .ins()
            .store(MemFlags::new(), data, new_closure_ptr, new_offset);
        builder
            .ins()
            .store(MemFlags::new(), tag, new_closure_ptr, new_offset + 8);
    }

    // Append the new argument.
    let (new_arg_data, new_arg_tag) = args.new_arg;
    let new_arg_offset = ClosureLayout::applied_data_offset(free_len, args.incoming_applied_len);
    builder.ins().store(
        MemFlags::new(),
        new_arg_data,
        new_closure_ptr,
        new_arg_offset,
    );
    builder.ins().store(
        MemFlags::new(),
        new_arg_tag,
        new_closure_ptr,
        new_arg_offset + 8,
    );

    closure_const(builder, new_closure_ptr)
}

fn free_variables(
    expr: &Expr,
    module_names: &HashSet<String>,
    bound: &HashSet<String>,
) -> HashSet<String> {
    let mut free = HashSet::new();
    collect_free_variables(expr, module_names, bound, &mut free);
    free
}

fn collect_free_variables(
    expr: &Expr,
    module_names: &HashSet<String>,
    bound: &HashSet<String>,
    free: &mut HashSet<String>,
) {
    match expr {
        Expr::Ident(name) => {
            if !bound.contains(name) && !module_names.contains(name) {
                free.insert(name.clone());
            }
        }
        Expr::Number(_) => {}
        Expr::Apply { func, arg } => {
            collect_free_variables(func, module_names, bound, free);
            collect_free_variables(arg, module_names, bound, free);
        }
        Expr::Lambda { params, body } => {
            let mut new_bound = bound.clone();
            for param in params {
                new_bound.insert(param.clone());
            }
            collect_free_variables(body, module_names, &new_bound, free);
        }
        Expr::Infix { left, right, .. } => {
            collect_free_variables(left, module_names, bound, free);
            collect_free_variables(right, module_names, bound, free);
        }
        Expr::Prefix { expr, .. } => {
            collect_free_variables(expr, module_names, bound, free);
        }
        Expr::Access { target, .. } => {
            collect_free_variables(target, module_names, bound, free);
        }
        Expr::Record(fields) => {
            for field in fields {
                collect_free_variables(&field.value, module_names, bound, free);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Binding, LetBlock};

    #[test]
    fn jit_lambda_and_application() {
        // vars
        //   closure = x y => x * y
        //   answer = closure 6 7
        // expr
        //   answer
        let ast = Ast(vec![Block::Let(LetBlock {
            bindings: vec![
                Binding {
                    name: "closure".to_string(),
                    value: Expr::Lambda {
                        params: vec!["x".to_string(), "y".to_string()],
                        body: Box::new(Expr::Infix {
                            op: InfixOp::Mul,
                            left: Box::new(Expr::Ident("x".to_string())),
                            right: Box::new(Expr::Ident("y".to_string())),
                        }),
                    },
                },
                Binding {
                    name: "answer".to_string(),
                    value: Expr::Apply {
                        func: Box::new(Expr::Apply {
                            func: Box::new(Expr::Ident("closure".to_string())),
                            arg: Box::new(Expr::Number("6".to_string())),
                        }),
                        arg: Box::new(Expr::Number("7".to_string())),
                    },
                },
            ],
            body: Box::new(Expr::Ident("answer".to_string())),
        })]);

        let func = Compiler::new().unwrap().compile(&ast).unwrap();
        let (value, tag) = func();
        assert_eq!(tag, TAG_NUMBER);
        assert_eq!(value, 42);
    }

    #[test]
    fn jit_lambda_capture() {
        // vars
        //   make_add = n => x => n + x
        //   add5 = make_add 5
        //   answer = add5 10
        // expr
        //   answer
        let ast = Ast(vec![Block::Let(LetBlock {
            bindings: vec![
                Binding {
                    name: "make_add".to_string(),
                    value: Expr::Lambda {
                        params: vec!["n".to_string()],
                        body: Box::new(Expr::Lambda {
                            params: vec!["x".to_string()],
                            body: Box::new(Expr::Infix {
                                op: InfixOp::Add,
                                left: Box::new(Expr::Ident("n".to_string())),
                                right: Box::new(Expr::Ident("x".to_string())),
                            }),
                        }),
                    },
                },
                Binding {
                    name: "add5".to_string(),
                    value: Expr::Apply {
                        func: Box::new(Expr::Ident("make_add".to_string())),
                        arg: Box::new(Expr::Number("5".to_string())),
                    },
                },
                Binding {
                    name: "answer".to_string(),
                    value: Expr::Apply {
                        func: Box::new(Expr::Ident("add5".to_string())),
                        arg: Box::new(Expr::Number("10".to_string())),
                    },
                },
            ],
            body: Box::new(Expr::Ident("answer".to_string())),
        })]);

        let func = Compiler::new().unwrap().compile(&ast).unwrap();
        let (value, tag) = func();
        assert_eq!(tag, TAG_NUMBER);
        assert_eq!(value, 15);
    }
}
