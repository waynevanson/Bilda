use cranelift::prelude::{Type, types};
use cranelift_jit::JITModule;
use cranelift_module::Module;

#[derive(Clone, Copy)]
pub struct Types {
    pub pointer: Type,
    pub int: Type,
    pub bool: Type,
}

impl Types {
    pub fn from_module(module: &JITModule) -> Self {
        let pointer = module.target_config().pointer_type();
        Self {
            pointer,
            int: pointer,
            bool: types::I8,
        }
    }
}
