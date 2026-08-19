use std::fmt;

#[derive(Debug)]
pub enum CompileError {
    UnknownReference(String),
    Unsupported(String),
    Module(String),
    Settings(String),
    Codegen(String),
    Internal(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::UnknownReference(name) => write!(f, "unknown reference: {name}"),
            CompileError::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            CompileError::Module(msg) => write!(f, "module error: {msg}"),
            CompileError::Settings(msg) => write!(f, "settings error: {msg}"),
            CompileError::Codegen(msg) => write!(f, "codegen error: {msg}"),
            CompileError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<cranelift_module::ModuleError> for CompileError {
    fn from(err: cranelift_module::ModuleError) -> Self {
        CompileError::Module(err.to_string())
    }
}
