use std::mem;

use cranelift::codegen::Context;
use cranelift::codegen::ir::{FuncRef, Function, UserFuncName};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

use crate::ast::{
    Ast, Boolean, Call, Expression, Lambda, LetIn, Map, Math, MathSign, MathTarget, Product, Sum,
};
use crate::jit::collect::value_is_function;
use crate::jit::compiled::Compiled;
use crate::jit::env::Env;
use crate::jit::error::CompileError;
use crate::jit::helpers::RuntimeHelpers;
use crate::jit::lambdas::{LambdaTable, Lambdas};
use crate::jit::symbols::register_runtime_symbols;
use crate::jit::types::Types;
use crate::runtime::RawValue;

const VALUE_PAYLOAD_OFFSET: i32 = mem::offset_of!(RawValue, payload) as i32;

pub struct Compiler<'a> {
    pub module: JITModule,
    pub helpers: RuntimeHelpers,
    pub types: Types,
    pub lambda_table: LambdaTable<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new() -> Result<Self, CompileError> {
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
        let types = Types::from_module(&module);
        let helpers = RuntimeHelpers::declare(&mut module, types)?;

        Ok(Self {
            module,
            helpers,
            types,
            lambda_table: LambdaTable::empty(),
        })
    }

    fn lambda_signature(&mut self) -> Signature {
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(self.types.pointer));
        sig.params.push(AbiParam::new(self.types.pointer));
        sig.returns.push(AbiParam::new(self.types.pointer));
        sig
    }

    pub fn compile_lambdas(
        &mut self,
        lambdas: Lambdas<'a>,
        ctx: &mut Context,
        builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<(), CompileError> {
        let Lambdas {
            expressions,
            func_ids,
        } = lambdas;
        self.lambda_table = LambdaTable { func_ids };

        for &lambda_expr in &expressions {
            let lambda = match unsafe { &*lambda_expr } {
                Expression::Lambda(l) => l,
                _ => unreachable!(),
            };

            if lambda.params.len() != 1 {
                return Err(CompileError::Unsupported(
                    "only single-parameter lambdas are supported".into(),
                ));
            }

            let func_id = self.lambda_table.func_ids[&lambda_expr];
            ctx.func.signature = self.lambda_signature();
            ctx.func.name = UserFuncName::user(0, func_id.as_u32());

            self.compile_lambda_body(&mut ctx.func, builder_ctx, lambda)?;

            self.module.define_function(func_id, ctx)?;
            self.module.clear_context(ctx);
        }

        Ok(())
    }

    fn compile_lambda_body(
        &mut self,
        func: &mut Function,
        builder_ctx: &mut FunctionBuilderContext,
        lambda: &'a Lambda<'a>,
    ) -> Result<(), CompileError> {
        let mut bcx = FunctionBuilder::new(func, builder_ctx);
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
        Ok(())
    }

    fn compile_main_body(
        &mut self,
        func: &mut Function,
        builder_ctx: &mut FunctionBuilderContext,
        ast: &'a Ast<'a>,
    ) -> Result<(), CompileError> {
        let mut bcx = FunctionBuilder::new(func, builder_ctx);
        let block = bcx.create_block();
        bcx.switch_to_block(block);
        bcx.append_block_params_for_function_params(block);

        let mut env = Env::new();
        env.push_scope();

        let result = self.compile_ast(&mut bcx, ast, &mut env)?;
        bcx.ins().return_(&[result]);
        bcx.seal_all_blocks();
        bcx.finalize();
        Ok(())
    }

    pub fn compile_main(
        mut self,
        ast: &'a Ast<'a>,
        ctx: &mut Context,
        builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<Compiled, CompileError> {
        let mut main_sig = self.module.make_signature();
        main_sig.returns.push(AbiParam::new(self.types.pointer));
        let main_id = self
            .module
            .declare_function("main", Linkage::Export, &main_sig)?;

        ctx.func.signature = main_sig;
        ctx.func.name = UserFuncName::user(0, main_id.as_u32());

        self.compile_main_body(&mut ctx.func, builder_ctx, ast)?;

        self.module.define_function(main_id, ctx)?;
        self.module.finalize_definitions()?;

        let code = self.module.get_finalized_function(main_id);
        let main = unsafe { mem::transmute::<*const u8, extern "C" fn() -> *mut RawValue>(code) };

        Ok(Compiled::new(self.module, main))
    }

    fn helper_ref(&mut self, bcx: &mut FunctionBuilder, name: &'static str) -> FuncRef {
        self.helpers.declare_in_func(&mut self.module, bcx, name)
    }

    pub fn call_helper(
        &mut self,
        bcx: &mut FunctionBuilder,
        name: &'static str,
        args: &[Value],
    ) -> Result<Value, CompileError> {
        let func_ref = self.helper_ref(bcx, name);
        let call = bcx.ins().call(func_ref, args);
        let results = bcx.inst_results(call);
        if results.is_empty() {
            Ok(bcx.ins().iconst(self.types.pointer, 0))
        } else {
            Ok(results[0])
        }
    }

    fn emit_string_const(&mut self, bcx: &mut FunctionBuilder, s: &'a str) -> (Value, Value) {
        let ptr = bcx.ins().iconst(self.types.pointer, s.as_ptr() as i64);
        let len = bcx.ins().iconst(self.types.int, s.len() as i64);
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
                let c = bcx.ins().iconst(self.types.int, *n as i64);
                self.call_helper(bcx, "bilda_make_int", &[c])
            }
            Expression::Float(f) => {
                let c = bcx.ins().f64const(*f);
                self.call_helper(bcx, "bilda_make_float", &[c])
            }
            Expression::Boolean(Boolean(b)) => {
                let c = bcx.ins().iconst(self.types.bool, if *b { 1 } else { 0 });
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
            Expression::Not(Boolean(b)) => {
                let c = bcx.ins().iconst(self.types.bool, if *b { 0 } else { 1 });
                self.call_helper(bcx, "bilda_make_bool", &[c])
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
                let func_id = self
                    .lambda_table
                    .get(key)
                    .ok_or_else(|| CompileError::Unsupported("lambda not collected".to_string()))?;
                let func_ref = self.module.declare_func_in_func(func_id, bcx.func);
                let func_addr = bcx.ins().func_addr(self.types.pointer, func_ref);
                let null_env = bcx.ins().iconst(self.types.pointer, 0);
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
                let zero = bcx.ins().iconst(self.types.pointer, 0);
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

        let left_val = bcx
            .ins()
            .load(self.types.int, MemFlags::new(), left, VALUE_PAYLOAD_OFFSET);
        let right_val =
            bcx.ins()
                .load(self.types.int, MemFlags::new(), right, VALUE_PAYLOAD_OFFSET);

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
                let c = bcx.ins().iconst(self.types.int, *n as i64);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::compiled::compile;
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

    #[test]
    fn compile_not() {
        let ast = parse("!True");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_BOOL);
        assert_eq!(v.payload, 0);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };

        let ast = parse("!False");
        let compiled = compile(&ast).unwrap();
        let v = unsafe { &*compiled.run() };
        assert_eq!(v.tag, TAG_BOOL);
        assert_eq!(v.payload, 1);
        unsafe { bilda_decref(v as *const RawValue as *mut RawValue) };
    }
}
