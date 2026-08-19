use std::collections::HashMap;
use std::mem;

use cranelift::codegen::Context;
use cranelift::codegen::ir::condcodes::IntCC;
use cranelift::codegen::ir::{FuncRef, UserFuncName};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};

use crate::ast::{Ast, Call, Expression, LetIn, Map, Math, MathSign, MathTarget, Product, Sum};
use crate::runtime::{
    RawValue, bilda_alloc_map, bilda_apply, bilda_decref, bilda_incref, bilda_make_bool,
    bilda_make_closure, bilda_make_float, bilda_make_int, bilda_make_string, bilda_map_rename,
    bilda_map_set, bilda_print,
};

/// Byte offset of `RawValue::payload` in a `Value` struct.
const VALUE_PAYLOAD_OFFSET: i32 = mem::offset_of!(RawValue, payload) as i32;

#[derive(Debug)]
pub enum CompileError {
    UnknownReference(String),
    Unsupported(String),
    Module(String),
}

impl From<cranelift_module::ModuleError> for CompileError {
    fn from(err: cranelift_module::ModuleError) -> Self {
        CompileError::Module(err.to_string())
    }
}

pub struct Compiled {
    #[allow(dead_code)]
    module: JITModule,
    main: extern "C" fn() -> *mut RawValue,
}

impl Compiled {
    pub fn run(&self) -> *mut RawValue {
        (self.main)()
    }
}

pub fn compile(ast: &Ast) -> Result<Compiled, CompileError> {
    let mut compiler = Compiler::new()?;
    let mut ctx = compiler.module.make_context();
    let mut builder_ctx = FunctionBuilderContext::new();

    compiler.declare_lambdas(ast)?;
    compiler.compile_lambdas(&mut ctx, &mut builder_ctx)?;
    compiler.compile_main(ast, &mut ctx, &mut builder_ctx)
}

struct Compiler<'a> {
    module: JITModule,
    helpers: HashMap<&'static str, FuncId>,
    lambda_funcs: HashMap<*const Expression<'a>, FuncId>,
    lambdas: Vec<*const Expression<'a>>,
    pointer_type: Type,
    int_type: Type,
    bool_type: Type,
}

impl<'a> Compiler<'a> {
    fn new() -> Result<Self, CompileError> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {msg}");
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
        register_runtime_symbols(&mut builder);

        let module = JITModule::new(builder);
        let pointer_type = module.target_config().pointer_type();
        let int_type = pointer_type;
        let bool_type = types::I8;

        let mut compiler = Self {
            module,
            helpers: HashMap::new(),
            lambda_funcs: HashMap::new(),
            lambdas: Vec::new(),
            pointer_type,
            int_type,
            bool_type,
        };
        compiler.declare_runtime_helpers()?;
        Ok(compiler)
    }

    fn declare_runtime_helpers(&mut self) -> Result<(), CompileError> {
        let pointer_type = self.pointer_type;
        let int_type = self.int_type;
        let bool_type = self.bool_type;

        let specs: [(&'static str, &[Type], &[Type]); 12] = [
            ("bilda_make_int", &[int_type], &[pointer_type]),
            ("bilda_make_bool", &[bool_type], &[pointer_type]),
            ("bilda_make_float", &[types::F64], &[pointer_type]),
            ("bilda_make_string", &[pointer_type, int_type], &[pointer_type]),
            ("bilda_alloc_map", &[pointer_type, int_type], &[pointer_type]),
            ("bilda_map_rename", &[pointer_type, pointer_type, int_type], &[]),
            (
                "bilda_map_set",
                &[pointer_type, pointer_type, int_type, pointer_type],
                &[],
            ),
            ("bilda_make_closure", &[pointer_type, pointer_type], &[pointer_type]),
            ("bilda_apply", &[pointer_type, pointer_type], &[pointer_type]),
            ("bilda_incref", &[pointer_type], &[]),
            ("bilda_decref", &[pointer_type], &[]),
            ("bilda_print", &[pointer_type], &[]),
        ];

        for (name, params, returns) in specs {
            let mut sig = self.module.make_signature();
            for &ty in params {
                sig.params.push(AbiParam::new(ty));
            }
            for &ty in returns {
                sig.returns.push(AbiParam::new(ty));
            }
            let id = self.module.declare_function(name, Linkage::Import, &sig)?;
            self.helpers.insert(name, id);
        }

        Ok(())
    }

    fn lambda_signature(&mut self) -> Signature {
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(self.pointer_type));
        sig.params.push(AbiParam::new(self.pointer_type));
        sig.returns.push(AbiParam::new(self.pointer_type));
        sig
    }

    fn declare_lambdas(&mut self, ast: &Ast<'a>) -> Result<(), CompileError> {
        collect_lambdas_ast(ast, &mut self.lambdas);

        for idx in 0..self.lambdas.len() {
            let lambda_expr = self.lambdas[idx];
            let sig = self.lambda_signature();
            let name = format!("lambda_{idx}");
            let id = self.module.declare_function(&name, Linkage::Local, &sig)?;
            self.lambda_funcs.insert(lambda_expr, id);
        }

        Ok(())
    }

    fn compile_lambdas(
        &mut self,
        ctx: &mut Context,
        builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<(), CompileError> {
        let lambdas = self.lambdas.clone();

        for &lambda_expr in &lambdas {
            let lambda = match unsafe { &*lambda_expr } {
                Expression::Lambda(l) => l,
                _ => unreachable!(),
            };

            if lambda.params.len() != 1 {
                return Err(CompileError::Unsupported(
                    "only single-parameter lambdas are supported".into(),
                ));
            }

            let func_id = self.lambda_funcs[&lambda_expr];
            ctx.func.signature = self.lambda_signature();
            ctx.func.name = UserFuncName::user(0, func_id.as_u32());

            {
                let mut bcx = FunctionBuilder::new(&mut ctx.func, builder_ctx);
                let block = bcx.create_block();
                bcx.switch_to_block(block);
                bcx.append_block_params_for_function_params(block);
                let arg_param = bcx.block_params(block)[0];

                let mut env = Env::new();
                env.push_scope();
                env.insert(lambda.params[0], arg_param, false);

                let body = self.compile_expr(&mut bcx, &lambda.body, &mut env)?;
                env.decref_scope_except(&mut bcx, self, body)?;
                bcx.ins().return_(&[body]);
                bcx.seal_all_blocks();
                bcx.finalize();
            }

            self.module.define_function(func_id, ctx)?;
            self.module.clear_context(ctx);
        }

        Ok(())
    }

    fn compile_main(
        mut self,
        ast: &'a Ast<'a>,
        ctx: &mut Context,
        builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<Compiled, CompileError> {
        let mut main_sig = self.module.make_signature();
        main_sig.returns.push(AbiParam::new(self.pointer_type));
        let main_id = self.module.declare_function("main", Linkage::Export, &main_sig)?;

        ctx.func.signature = main_sig;
        ctx.func.name = UserFuncName::user(0, main_id.as_u32());

        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, builder_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.append_block_params_for_function_params(block);

            let mut env = Env::new();
            env.push_scope();

            let result = self.compile_ast(&mut bcx, ast, &mut env)?;
            bcx.ins().return_(&[result]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }

        self.module.define_function(main_id, ctx)?;
        self.module.finalize_definitions()?;

        let code = self.module.get_finalized_function(main_id);
        let main = unsafe { mem::transmute::<*const u8, extern "C" fn() -> *mut RawValue>(code) };

        Ok(Compiled {
            module: self.module,
            main,
        })
    }

    fn helper_ref(&mut self, bcx: &mut FunctionBuilder, name: &'static str) -> FuncRef {
        let id = self.helpers[name];
        self.module.declare_func_in_func(id, bcx.func)
    }

    fn call_helper(
        &mut self,
        bcx: &mut FunctionBuilder,
        name: &'static str,
        args: &[Value],
    ) -> Result<Value, CompileError> {
        let func_ref = self.helper_ref(bcx, name);
        let call = bcx.ins().call(func_ref, args);
        let results = bcx.inst_results(call);
        if results.is_empty() {
            Ok(bcx.ins().iconst(self.pointer_type, 0))
        } else {
            Ok(results[0])
        }
    }

    fn emit_string_const(&mut self, bcx: &mut FunctionBuilder, s: &'a str) -> (Value, Value) {
        let ptr = bcx.ins().iconst(self.pointer_type, s.as_ptr() as i64);
        let len = bcx.ins().iconst(self.int_type, s.len() as i64);
        (ptr, len)
    }

    fn compile_ast(
        &mut self,
        bcx: &mut FunctionBuilder,
        ast: &'a Ast<'a>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        match ast {
            Ast::LetIn(LetIn {
                assignments,
                expression,
            }) => {
                env.push_scope();
                for assignment in assignments {
                    let value = self.compile_ast(bcx, &assignment.value, env)?;
                    let is_function = value_is_function(&assignment.value, env);
                    env.insert(assignment.name, value, is_function);
                }
                let result = self.compile_expr(bcx, expression, env)?;
                env.decref_scope_except(bcx, self, result)?;
                env.pop_scope();
                Ok(result)
            }
            Ast::Expression(expr) => self.compile_expr(bcx, expr, env),
        }
    }

    fn compile_expr(
        &mut self,
        bcx: &mut FunctionBuilder,
        expr: &'a Expression<'a>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        match expr {
            Expression::Int(n) => {
                let c = bcx.ins().iconst(self.int_type, *n as i64);
                self.call_helper(bcx, "bilda_make_int", &[c])
            }
            Expression::Float(f) => {
                let c = bcx.ins().f64const(*f);
                self.call_helper(bcx, "bilda_make_float", &[c])
            }
            Expression::Boolean(b) => {
                let c = bcx.ins().iconst(self.bool_type, if *b { 1 } else { 0 });
                self.call_helper(bcx, "bilda_make_bool", &[c])
            }
            Expression::String(s) => {
                let (ptr, len) = self.emit_string_const(bcx, s);
                self.call_helper(bcx, "bilda_make_string", &[ptr, len])
            }
            Expression::Reference(name) => {
                if *name == "String" || *name == "Int" {
                    let (ptr, len) = self.emit_string_const(bcx, name);
                    return self.call_helper(bcx, "bilda_make_string", &[ptr, len]);
                }
                match env.get(name) {
                    Some(slot) => {
                        self.call_helper(bcx, "bilda_incref", &[slot.value])?;
                        Ok(slot.value)
                    }
                    None => Err(CompileError::UnknownReference((*name).to_string())),
                }
            }
            Expression::Math(math) => self.compile_math(bcx, math, env),
            Expression::Map(Map { assignments }) => self.compile_map(bcx, assignments, None, env),
            Expression::Product(Product { assignments }) => {
                self.compile_map(bcx, assignments, None, env)
            }
            Expression::Sum(Sum { assignments }) => self.compile_map(bcx, assignments, None, env),
            Expression::Call(call) => self.compile_call(bcx, call, env),
            Expression::Lambda(_) => {
                let key = expr as *const Expression;
                let func_id = self.lambda_funcs.get(&key).copied().ok_or_else(|| {
                    CompileError::Unsupported("lambda not collected".to_string())
                })?;
                let func_ref = self.module.declare_func_in_func(func_id, bcx.func);
                let func_addr = bcx.ins().func_addr(self.pointer_type, func_ref);
                let null_env = bcx.ins().iconst(self.pointer_type, 0);
                self.call_helper(bcx, "bilda_make_closure", &[func_addr, null_env])
            }
        }
    }

    fn compile_map(
        &mut self,
        bcx: &mut FunctionBuilder,
        assignments: &'a [crate::ast::Assignment<'a>],
        name: Option<&'a str>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        let (name_ptr, name_len) = match name {
            Some(s) => self.emit_string_const(bcx, s),
            None => {
                let zero = bcx.ins().iconst(self.pointer_type, 0);
                (zero, zero)
            }
        };
        let map = self.call_helper(bcx, "bilda_alloc_map", &[name_ptr, name_len])?;
        for assignment in assignments {
            let value = self.compile_ast(bcx, &assignment.value, env)?;
            let (field_ptr, field_len) = self.emit_string_const(bcx, assignment.name);
            self.call_helper(bcx, "bilda_map_set", &[map, field_ptr, field_len, value])?;
            self.call_helper(bcx, "bilda_decref", &[value])?;
        }
        Ok(map)
    }

    fn compile_call(
        &mut self,
        bcx: &mut FunctionBuilder,
        call: &'a Call<'a>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        let arg = self.compile_expr(bcx, &call.argument, env)?;

        match &*call.function {
            Expression::Reference(name) => {
                if let Some(slot) = env.get(name)
                    && slot.is_function
                {
                    self.call_helper(bcx, "bilda_incref", &[slot.value])?;
                    let result = self.call_helper(bcx, "bilda_apply", &[slot.value, arg])?;
                    self.call_helper(bcx, "bilda_decref", &[slot.value])?;
                    return Ok(result);
                }

                let (name_ptr, name_len) = self.emit_string_const(bcx, name);
                self.call_helper(bcx, "bilda_map_rename", &[arg, name_ptr, name_len])?;
                Ok(arg)
            }
            other => {
                let callee = self.compile_expr(bcx, other, env)?;
                let result = self.call_helper(bcx, "bilda_apply", &[callee, arg])?;
                self.call_helper(bcx, "bilda_decref", &[callee])?;
                Ok(result)
            }
        }
    }

    fn compile_math(
        &mut self,
        bcx: &mut FunctionBuilder,
        math: &'a Math<'a>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        let left = self.compile_math_target(bcx, &math.left, env)?;
        let right = self.compile_math_target(bcx, &math.right, env)?;

        let left_val =
            bcx.ins()
                .load(self.int_type, MemFlags::new(), left, VALUE_PAYLOAD_OFFSET);
        let right_val =
            bcx.ins()
                .load(self.int_type, MemFlags::new(), right, VALUE_PAYLOAD_OFFSET);

        let result = match math.sign {
            MathSign::Addition => bcx.ins().iadd(left_val, right_val),
            MathSign::Subtraction => bcx.ins().isub(left_val, right_val),
            MathSign::Multiplication => bcx.ins().imul(left_val, right_val),
            MathSign::Division => bcx.ins().sdiv(left_val, right_val),
        };

        let result = self.call_helper(bcx, "bilda_make_int", &[result])?;
        self.call_helper(bcx, "bilda_decref", &[left])?;
        self.call_helper(bcx, "bilda_decref", &[right])?;
        Ok(result)
    }

    fn compile_math_target(
        &mut self,
        bcx: &mut FunctionBuilder,
        target: &'a MathTarget<'a>,
        env: &mut Env<'a>,
    ) -> Result<Value, CompileError> {
        match target {
            MathTarget::Number(n) => {
                let c = bcx.ins().iconst(self.int_type, *n as i64);
                self.call_helper(bcx, "bilda_make_int", &[c])
            }
            MathTarget::Reference(name) => match env.get(name) {
                Some(slot) => {
                    self.call_helper(bcx, "bilda_incref", &[slot.value])?;
                    Ok(slot.value)
                }
                None => Err(CompileError::UnknownReference((*name).to_string())),
            },
            MathTarget::Math(math) => self.compile_math(bcx, math, env),
        }
    }
}

struct Slot {
    value: Value,
    is_function: bool,
}

struct Env<'a> {
    scopes: Vec<HashMap<&'a str, Slot>>,
    bindings: Vec<Vec<Value>>,
}

impl<'a> Env<'a> {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            bindings: vec![Vec::new()],
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.bindings.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.bindings.pop();
    }

    fn insert(&mut self, name: &'a str, value: Value, is_function: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, Slot { value, is_function });
        self.bindings.last_mut().unwrap().push(value);
    }

    fn get(&self, name: &str) -> Option<&Slot> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn decref_scope_except(
        &self,
        bcx: &mut FunctionBuilder,
        compiler: &mut Compiler<'a>,
        except: Value,
    ) -> Result<(), CompileError> {
        for &binding in self.bindings.last().unwrap() {
            let cond = bcx.ins().icmp(IntCC::Equal, binding, except);
            let skip_block = bcx.create_block();
            let drop_block = bcx.create_block();
            let merge_block = bcx.create_block();
            bcx.ins().brif(cond, skip_block, &[], drop_block, &[]);
            bcx.switch_to_block(drop_block);
            compiler.call_helper(bcx, "bilda_decref", &[binding])?;
            bcx.ins().jump(merge_block, &[]);
            bcx.switch_to_block(skip_block);
            bcx.ins().jump(merge_block, &[]);
            bcx.switch_to_block(merge_block);
            bcx.seal_block(skip_block);
            bcx.seal_block(drop_block);
            bcx.seal_block(merge_block);
        }
        Ok(())
    }
}

fn collect_lambdas_ast<'a>(ast: &Ast<'a>, out: &mut Vec<*const Expression<'a>>) {
    match ast {
        Ast::LetIn(LetIn {
            assignments,
            expression,
        }) => {
            for assignment in assignments {
                collect_lambdas_ast(&assignment.value, out);
            }
            collect_lambdas_expr(expression, out);
        }
        Ast::Expression(expr) => collect_lambdas_expr(expr, out),
    }
}

fn collect_lambdas_expr<'a>(expr: &Expression<'a>, out: &mut Vec<*const Expression<'a>>) {
    match expr {
        Expression::Lambda(lambda) => {
            out.push(expr as *const Expression<'a>);
            collect_lambdas_expr(&lambda.body, out);
        }
        Expression::Map(Map { assignments })
        | Expression::Product(Product { assignments })
        | Expression::Sum(Sum { assignments }) => {
            for assignment in assignments {
                collect_lambdas_ast(&assignment.value, out);
            }
        }
        Expression::Call(Call { function, argument }) => {
            collect_lambdas_expr(function, out);
            collect_lambdas_expr(argument, out);
        }
        Expression::Math(Math { left, right, .. }) => {
            collect_math_target(left, out);
            collect_math_target(right, out);
        }
        _ => {}
    }
}

#[allow(clippy::only_used_in_recursion)]
fn collect_math_target<'a>(target: &MathTarget<'a>, out: &mut Vec<*const Expression<'a>>) {
    if let MathTarget::Math(math) = target {
        collect_math_target(&math.left, out);
        collect_math_target(&math.right, out);
    }
}

fn value_is_function<'a>(ast: &Ast<'a>, env: &Env<'a>) -> bool {
    match ast {
        Ast::Expression(expr) => expr_is_function(expr, env),
        Ast::LetIn(LetIn { expression, .. }) => expr_is_function(expression, env),
    }
}

fn expr_is_function<'a>(expr: &Expression<'a>, env: &Env<'a>) -> bool {
    match expr {
        Expression::Lambda(_) => true,
        Expression::Reference(name) => env.get(name).map(|s| s.is_function).unwrap_or(false),
        _ => false,
    }
}

fn register_runtime_symbols(builder: &mut JITBuilder) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Token;
    use crate::parser::ast;
    use crate::runtime::bilda_decref;
    use crate::runtime::value::{MapObj, StringObj, TAG_BOOL, TAG_INT, TAG_MAP, TAG_STRING};
    use chumsky::Parser;
    use chumsky::input::Stream;
    use logos::Logos;

    fn parse(source: &str) -> crate::ast::Ast<'_> {
        let tokens: Vec<Token<'_>> = Token::lexer(source).collect::<Result<_, _>>().unwrap();
        ast()
            .parse(Stream::from_iter(tokens))
            .into_result()
            .unwrap()
    }

    #[test]
    fn compile_int() {
        let ast = parse("42");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 42);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }

    #[test]
    fn compile_math() {
        let ast = parse("(1 + 2) * 3");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 9);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }

    #[test]
    fn compile_let() {
        let ast = parse("let a = 1 b = 2 in a + b");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 3);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }

    #[test]
    fn compile_string() {
        let ast = parse(r#""hello""#);
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_STRING);
        let obj = unsafe { &*(v.payload as *const StringObj) };
        assert_eq!(obj.len, 5);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }

    #[test]
    fn compile_constructor() {
        let ast = parse(
            r#"
            let
              Status = + { completed = True }
            in
              Status { completed = False }
        "#,
        );
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_MAP);
        let obj = unsafe { &*(v.payload as *const MapObj) };
        let name = unsafe { std::ffi::CStr::from_ptr(obj.name) };
        assert_eq!(name.to_str().unwrap(), "Status");
        assert_eq!(obj.len, 1);
        let entry = unsafe { &*obj.entries };
        let field_name = unsafe { std::ffi::CStr::from_ptr(entry.name) };
        assert_eq!(field_name.to_str().unwrap(), "completed");
        assert_eq!(unsafe { (*entry.value).tag }, TAG_BOOL);
        assert_eq!(unsafe { (*entry.value).payload }, 0);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }

    #[test]
    fn compile_lambda() {
        let ast = parse("let f = (x: Int) => x + 1 in f(5)");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 6);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }
}
