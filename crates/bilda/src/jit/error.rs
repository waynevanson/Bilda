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
