use std::fmt;

#[derive(Debug)]
pub enum CompileError {
    Reference(ReferenceError),
    Unsupported(UnsupportedError),
    Module(Box<ModuleError>),
    Settings(SettingsError),
    Codegen(CodegenError),
    Internal(InternalError),
}

#[derive(Debug)]
pub enum ReferenceError {
    Unknown { name: String },
}

#[derive(Debug)]
pub enum UnsupportedError {
    LambdaArity { found: usize },
    LambdaNotCollected,
}

#[derive(Debug)]
pub enum ModuleError {
    Cranelift(cranelift_module::ModuleError),
}

#[derive(Debug)]
pub enum SettingsError {
    BadName { name: String },
    BadType,
    BadValue { value: String },
}

#[derive(Debug)]
pub enum CodegenError {
    HostUnsupported,
    SupportDisabled,
    MissingCpuFeature { feature: &'static str },
    BuildIsa(cranelift::codegen::CodegenError),
}

#[derive(Debug)]
pub enum InternalError {
    EnvScopeStackUnderflow,
    EnvBindingStackUnderflow,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Reference(err) => err.fmt(f),
            CompileError::Unsupported(err) => err.fmt(f),
            CompileError::Module(err) => err.fmt(f),
            CompileError::Settings(err) => err.fmt(f),
            CompileError::Codegen(err) => err.fmt(f),
            CompileError::Internal(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CompileError {}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceError::Unknown { name } => write!(f, "unknown reference: {name}"),
        }
    }
}

impl fmt::Display for UnsupportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnsupportedError::LambdaArity { found } => {
                write!(f, "only single-parameter lambdas are supported (found {found})")
            }
            UnsupportedError::LambdaNotCollected => write!(f, "lambda not collected"),
        }
    }
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleError::Cranelift(err) => err.fmt(f),
        }
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::BadName { name } => write!(f, "no setting named '{name}'"),
            SettingsError::BadType => write!(f, "wrong type for setting"),
            SettingsError::BadValue { value } => write!(f, "invalid value for setting: {value}"),
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::HostUnsupported => write!(f, "host architecture is unsupported"),
            CodegenError::SupportDisabled => {
                write!(f, "support for host architecture is disabled")
            }
            CodegenError::MissingCpuFeature { feature } => {
                write!(f, "missing required CPU feature: {feature}")
            }
            CodegenError::BuildIsa(err) => err.fmt(f),
        }
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::EnvScopeStackUnderflow => write!(f, "env scope stack underflow"),
            InternalError::EnvBindingStackUnderflow => write!(f, "env binding stack underflow"),
        }
    }
}

impl From<ReferenceError> for CompileError {
    fn from(err: ReferenceError) -> Self {
        CompileError::Reference(err)
    }
}

impl From<UnsupportedError> for CompileError {
    fn from(err: UnsupportedError) -> Self {
        CompileError::Unsupported(err)
    }
}

impl From<ModuleError> for CompileError {
    fn from(err: ModuleError) -> Self {
        CompileError::Module(Box::new(err))
    }
}

impl From<SettingsError> for CompileError {
    fn from(err: SettingsError) -> Self {
        CompileError::Settings(err)
    }
}

impl From<CodegenError> for CompileError {
    fn from(err: CodegenError) -> Self {
        CompileError::Codegen(err)
    }
}

impl From<InternalError> for CompileError {
    fn from(err: InternalError) -> Self {
        CompileError::Internal(err)
    }
}

impl From<cranelift_module::ModuleError> for CompileError {
    fn from(err: cranelift_module::ModuleError) -> Self {
        CompileError::Module(Box::new(ModuleError::Cranelift(err)))
    }
}

impl From<cranelift::codegen::settings::SetError> for CompileError {
    fn from(err: cranelift::codegen::settings::SetError) -> Self {
        let settings_err = match err {
            cranelift::codegen::settings::SetError::BadName(name) => SettingsError::BadName { name },
            cranelift::codegen::settings::SetError::BadType => SettingsError::BadType,
            cranelift::codegen::settings::SetError::BadValue(value) => {
                SettingsError::BadValue { value }
            }
        };
        CompileError::Settings(settings_err)
    }
}

impl From<cranelift::codegen::CodegenError> for CompileError {
    fn from(err: cranelift::codegen::CodegenError) -> Self {
        CompileError::Codegen(CodegenError::BuildIsa(err))
    }
}
