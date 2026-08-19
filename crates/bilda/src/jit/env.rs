use std::collections::HashMap;

use cranelift::prelude::{FunctionBuilder, InstBuilder, IntCC, Value};

use crate::jit::compiler::Compiler;
use crate::jit::error::CompileError;

pub(crate) struct Slot {
    pub value: Value,
    pub is_function: bool,
}

pub(crate) struct Env<'a> {
    scopes: Vec<HashMap<&'a str, Slot>>,
    bindings: Vec<Vec<Value>>,
}

impl<'a> Env<'a> {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            bindings: vec![Vec::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.bindings.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
        self.bindings.pop();
    }

    pub fn insert(
        &mut self,
        name: &'a str,
        value: Value,
        is_function: bool,
    ) -> Result<(), CompileError> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            CompileError::Internal("env scope stack underflow".into())
        })?;
        scope.insert(name, Slot { value, is_function });

        let bindings = self.bindings.last_mut().ok_or_else(|| {
            CompileError::Internal("env binding stack underflow".into())
        })?;
        bindings.push(value);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Slot> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn decref_scope_except(
        &self,
        bcx: &mut FunctionBuilder,
        compiler: &mut Compiler<'a>,
        except: Value,
    ) -> Result<(), CompileError> {
        let bindings = self.bindings.last().ok_or_else(|| {
            CompileError::Internal("env binding stack underflow".into())
        })?;
        for &binding in bindings {
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
