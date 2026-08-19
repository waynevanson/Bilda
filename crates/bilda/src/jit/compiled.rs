use cranelift::prelude::FunctionBuilderContext;
use cranelift_jit::JITModule;
use cranelift_module::Module;

use crate::ast::Ast;
use crate::jit::compiler::Compiler;
use crate::jit::error::CompileError;
use crate::jit::lambdas::Lambdas;
use crate::runtime::RawValue;

pub struct Compiled {
    #[allow(dead_code)]
    module: JITModule,
    main: extern "C" fn() -> *mut RawValue,
}

impl Compiled {
    pub(crate) fn new(module: JITModule, main: extern "C" fn() -> *mut RawValue) -> Self {
        Self { module, main }
    }

    pub fn run(&self) -> *mut RawValue {
        (self.main)()
    }
}

pub fn compile(ast: &Ast) -> Result<Compiled, CompileError> {
    let mut compiler = Compiler::new()?;
    let mut ctx = compiler.module.make_context();
    let mut builder_ctx = FunctionBuilderContext::new();

    let lambdas = Lambdas::collect(ast).declare(&mut compiler.module, compiler.types.pointer)?;
    compiler.compile_lambdas(lambdas, &mut ctx, &mut builder_ctx)?;
    compiler.compile_main(ast, &mut ctx, &mut builder_ctx)
}
