use std::collections::HashMap;
use std::mem;

use cranelift::codegen::ir::{FuncRef, UserFuncName};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};

use crate::ast::{Ast, Call, Expression, LetIn, Map, Math, MathSign, MathTarget, Product, Sum};
use crate::runtime::{
    RawValue, bilda_alloc_map, bilda_apply, bilda_decref, bilda_incref, bilda_make_bool,
    bilda_make_closure, bilda_make_float, bilda_make_int, bilda_make_string, bilda_map_rename,
    bilda_map_set, bilda_print, bilda_reset_arena,
};

type CValue = Value;

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
    unsafe {
        bilda_reset_arena();
    }

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

    let mut module = JITModule::new(builder);
    let pointer_type = module.target_config().pointer_type();
    let int_type = pointer_type;
    let float_type = types::F64;
    let bool_type = types::I8;

    let mut helpers = HashMap::new();
    declare_helpers(
        &mut module,
        &mut helpers,
        pointer_type,
        int_type,
        float_type,
        bool_type,
    )?;

    let mut lambda_funcs: HashMap<*const Expression, FuncId> = HashMap::new();
    let mut lambdas: Vec<*const Expression> = Vec::new();
    collect_lambdas_ast(ast, &mut lambdas);

    for (idx, lambda_expr) in lambdas.iter().enumerate() {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let name = format!("lambda_{idx}");
        let id = module.declare_function(&name, Linkage::Local, &sig)?;
        lambda_funcs.insert(*lambda_expr, id);
    }

    let mut ctx = module.make_context();
    let mut builder_ctx = FunctionBuilderContext::new();

    for lambda_expr in &lambdas {
        let lambda = match unsafe { &**lambda_expr } {
            Expression::Lambda(l) => l,
            _ => unreachable!(),
        };
        let func_id = lambda_funcs[lambda_expr];

        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));

        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());

        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
            let block = bcx.create_block();
            bcx.switch_to_block(block);
            bcx.append_block_params_for_function_params(block);
            let arg_param = bcx.block_params(block)[0];

            if lambda.params.len() != 1 {
                return Err(CompileError::Unsupported(
                    "only single-parameter lambdas are supported".into(),
                ));
            }

            let mut env = Env::new();
            env.push_scope();
            env.insert(lambda.params[0], arg_param, false);

            let mut fcx = FuncCx {
                module: &mut module,
                helpers: &helpers,
                lambda_funcs: &lambda_funcs,
                pointer_type,
                int_type,
                bool_type,
            };
            let body = fcx.compile_expr(&mut bcx, &lambda.body, &mut env)?;
            bcx.ins().return_(&[body]);
            bcx.seal_all_blocks();
            bcx.finalize();
        }

        module.define_function(func_id, &mut ctx)?;
        module.clear_context(&mut ctx);
    }

    let mut main_sig = module.make_signature();
    main_sig.returns.push(AbiParam::new(pointer_type));
    let main_id = module.declare_function("main", Linkage::Export, &main_sig)?;

    ctx.func.signature = main_sig;
    ctx.func.name = UserFuncName::user(0, main_id.as_u32());

    {
        let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.append_block_params_for_function_params(block);

        let mut env = Env::new();
        env.push_scope();

        let mut fcx = FuncCx {
            module: &mut module,
            helpers: &helpers,
            lambda_funcs: &lambda_funcs,
            pointer_type,
            int_type,
            bool_type,
        };
        let result = fcx.compile_ast(&mut bcx, ast, &mut env)?;
        bcx.ins().return_(&[result]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }

    module.define_function(main_id, &mut ctx)?;
    module.finalize_definitions()?;

    let code = module.get_finalized_function(main_id);
    let main = unsafe { mem::transmute::<*const u8, extern "C" fn() -> *mut RawValue>(code) };

    Ok(Compiled { module, main })
}

struct Slot {
    value: CValue,
    is_function: bool,
}

struct Env<'a> {
    scopes: Vec<HashMap<&'a str, Slot>>,
}

impl<'a> Env<'a> {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert(&mut self, name: &'a str, value: CValue, is_function: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, Slot { value, is_function });
    }

    fn get(&self, name: &str) -> Option<&Slot> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
}

struct FuncCx<'a, 'm> {
    module: &'m mut JITModule,
    helpers: &'a HashMap<&'static str, FuncId>,
    lambda_funcs: &'a HashMap<*const Expression<'a>, FuncId>,
    pointer_type: Type,
    int_type: Type,
    bool_type: Type,
}

impl<'a, 'm> FuncCx<'a, 'm> {
    fn helper_ref(&mut self, bcx: &mut FunctionBuilder, name: &'static str) -> FuncRef {
        let id = self.helpers[name];
        self.module.declare_func_in_func(id, bcx.func)
    }

    fn call_helper(
        &mut self,
        bcx: &mut FunctionBuilder,
        name: &'static str,
        args: &[CValue],
    ) -> Result<CValue, CompileError> {
        let func_ref = self.helper_ref(bcx, name);
        let call = bcx.ins().call(func_ref, args);
        let results = bcx.inst_results(call);
        if results.is_empty() {
            // Void helper: return a dummy value.
            Ok(bcx.ins().iconst(self.pointer_type, 0))
        } else {
            Ok(results[0])
        }
    }

    fn compile_ast(
        &mut self,
        bcx: &mut FunctionBuilder,
        ast: &'a Ast<'a>,
        env: &mut Env<'a>,
    ) -> Result<CValue, CompileError> {
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
    ) -> Result<CValue, CompileError> {
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
                let ptr = bcx.ins().iconst(self.pointer_type, s.as_ptr() as i64);
                let len = bcx.ins().iconst(self.int_type, s.len() as i64);
                self.call_helper(bcx, "bilda_make_string", &[ptr, len])
            }
            Expression::Reference(name) => {
                if *name == "String" || *name == "Int" {
                    let ptr = bcx.ins().iconst(self.pointer_type, name.as_ptr() as i64);
                    let len = bcx.ins().iconst(self.int_type, name.len() as i64);
                    return self.call_helper(bcx, "bilda_make_string", &[ptr, len]);
                }
                match env.get(name) {
                    Some(slot) => Ok(slot.value),
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
                let func_id =
                    self.lambda_funcs.get(&key).copied().ok_or_else(|| {
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
    ) -> Result<CValue, CompileError> {
        let (name_ptr, name_len) = match name {
            Some(s) => (
                bcx.ins().iconst(self.pointer_type, s.as_ptr() as i64),
                bcx.ins().iconst(self.int_type, s.len() as i64),
            ),
            None => (
                bcx.ins().iconst(self.pointer_type, 0),
                bcx.ins().iconst(self.int_type, 0),
            ),
        };
        let map = self.call_helper(bcx, "bilda_alloc_map", &[name_ptr, name_len])?;
        for assignment in assignments {
            let value = self.compile_ast(bcx, &assignment.value, env)?;
            let field_ptr = bcx
                .ins()
                .iconst(self.pointer_type, assignment.name.as_ptr() as i64);
            let field_len = bcx
                .ins()
                .iconst(self.int_type, assignment.name.len() as i64);
            self.call_helper(bcx, "bilda_map_set", &[map, field_ptr, field_len, value])?;
        }
        Ok(map)
    }

    fn compile_call(
        &mut self,
        bcx: &mut FunctionBuilder,
        call: &'a Call<'a>,
        env: &mut Env<'a>,
    ) -> Result<CValue, CompileError> {
        let arg = self.compile_expr(bcx, &call.argument, env)?;

        match &*call.function {
            Expression::Reference(name) => {
                if let Some(slot) = env.get(name)
                    && slot.is_function
                {
                    return self.call_helper(bcx, "bilda_apply", &[slot.value, arg]);
                }

                let name_ptr = bcx.ins().iconst(self.pointer_type, name.as_ptr() as i64);
                let name_len = bcx.ins().iconst(self.int_type, name.len() as i64);
                self.call_helper(bcx, "bilda_map_rename", &[arg, name_ptr, name_len])?;
                Ok(arg)
            }
            other => {
                let callee = self.compile_expr(bcx, other, env)?;
                self.call_helper(bcx, "bilda_apply", &[callee, arg])
            }
        }
    }

    fn compile_math(
        &mut self,
        bcx: &mut FunctionBuilder,
        math: &'a Math<'a>,
        env: &mut Env<'a>,
    ) -> Result<CValue, CompileError> {
        let left = self.compile_math_target(bcx, &math.left, env)?;
        let right = self.compile_math_target(bcx, &math.right, env)?;

        let left_val = bcx.ins().load(self.int_type, MemFlags::new(), left, 8);
        let right_val = bcx.ins().load(self.int_type, MemFlags::new(), right, 8);

        let result = match math.sign {
            MathSign::Addition => bcx.ins().iadd(left_val, right_val),
            MathSign::Subtraction => bcx.ins().isub(left_val, right_val),
            MathSign::Multiplication => bcx.ins().imul(left_val, right_val),
            MathSign::Division => bcx.ins().sdiv(left_val, right_val),
        };

        self.call_helper(bcx, "bilda_make_int", &[result])
    }

    fn compile_math_target(
        &mut self,
        bcx: &mut FunctionBuilder,
        target: &'a MathTarget<'a>,
        env: &mut Env<'a>,
    ) -> Result<CValue, CompileError> {
        match target {
            MathTarget::Number(n) => {
                let c = bcx.ins().iconst(self.int_type, *n as i64);
                self.call_helper(bcx, "bilda_make_int", &[c])
            }
            MathTarget::Reference(name) => match env.get(name) {
                Some(slot) => Ok(slot.value),
                None => Err(CompileError::UnknownReference((*name).to_string())),
            },
            MathTarget::Math(math) => self.compile_math(bcx, math, env),
        }
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
    builder.symbol("bilda_reset_arena", bilda_reset_arena as *const u8);
}

fn declare_helpers(
    module: &mut JITModule,
    helpers: &mut HashMap<&'static str, FuncId>,
    pointer_type: Type,
    int_type: Type,
    float_type: Type,
    bool_type: Type,
) -> Result<(), CompileError> {
    let mut sigs: HashMap<&'static str, Signature> = HashMap::new();

    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(int_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_make_int", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(bool_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_make_bool", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(float_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_make_float", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(int_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_make_string", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(int_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_alloc_map", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(int_type));
        sigs.insert("bilda_map_rename", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(int_type));
        sig.params.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_map_set", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_make_closure", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_apply", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_incref", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_decref", sig);
    }
    {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sigs.insert("bilda_print", sig);
    }

    for (name, sig) in sigs {
        let id = module.declare_function(name, Linkage::Import, &sig)?;
        helpers.insert(name, id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Token;
    use crate::parser::ast;
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
    }

    #[test]
    fn compile_math() {
        let ast = parse("(1 + 2) * 3");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 9);
    }

    #[test]
    fn compile_let() {
        let ast = parse("let a = 1 b = 2 in a + b");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 3);
    }

    #[test]
    fn compile_string() {
        let ast = parse(r#""hello""#);
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_STRING);
        let obj = unsafe { &*(v.payload as *const StringObj) };
        assert_eq!(obj.len, 5);
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
    }

    #[test]
    fn compile_lambda() {
        let ast = parse("let f = (x: Int) => x + 1 in f(5)");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_INT);
        assert_eq!(v.payload as isize, 6);
    }
}
