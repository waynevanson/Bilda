use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast::*;

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    Unit,
    TypeConstructor(String),
    TypeInstance(String, Box<Value>),
    Closure {
        params: Vec<String>,
        body: Expr,
        env: Rc<Env>,
    },
    Builtin(&'static str, fn(Value) -> Result<Value, RuntimeError>),
    Record(Vec<(String, Value)>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Unit => write!(f, "()"),
            Value::TypeConstructor(name) => write!(f, "<type {name}>"),
            Value::TypeInstance(_, value) => write!(f, "{value}"),
            Value::Closure { .. } => write!(f, "<closure>"),
            Value::Builtin(name, _) => write!(f, "<builtin {name}>"),
            Value::Record(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(name, value)| format!("{name} = {value}"))
                    .collect();
                write!(f, "{{ {} }}", parts.join(", "))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError(pub String);

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Debug, Clone)]
pub struct Env {
    parent: Option<Rc<Env>>,
    bindings: HashMap<String, Value>,
}

impl Env {
    fn new() -> Rc<Self> {
        Rc::new(Env {
            parent: None,
            bindings: HashMap::new(),
        })
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.bindings
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.get(name))
    }
}

fn define(env: Rc<Env>, name: String, value: Value) -> Rc<Env> {
    let mut bindings = HashMap::new();
    bindings.insert(name, value);
    Rc::new(Env {
        parent: Some(env),
        bindings,
    })
}

pub fn run(ast: &Ast) -> Result<Value, RuntimeError> {
    let mut env = Env::new();
    let mut result = Value::Unit;

    for block in &ast.0 {
        match block {
            Block::Use(block) => env = eval_use(block, env)?,
            Block::Type(block) => env = eval_type(block, env)?,
            Block::Let(block) => {
                let (new_env, value) = eval_let(block, env)?;
                env = new_env;
                result = value;
            }
        }
    }

    Ok(result)
}

fn eval_use(block: &UseBlock, env: Rc<Env>) -> Result<Rc<Env>, RuntimeError> {
    let mut env = env;
    for import in &block.imports {
        for name in &import.names {
            let value = resolve_import(&import.module, name)?;
            env = define(env, name.clone(), value);
        }
    }
    Ok(env)
}

fn resolve_import(module: &str, name: &str) -> Result<Value, RuntimeError> {
    match (module, name) {
        ("std", "echo") => Ok(Value::Builtin("echo", builtin_echo)),
        _ => Err(RuntimeError(format!(
            "unknown import {name} from module {module}"
        ))),
    }
}

fn builtin_echo(arg: Value) -> Result<Value, RuntimeError> {
    println!("{arg}");
    Ok(Value::Unit)
}

fn eval_type(block: &TypeBlock, env: Rc<Env>) -> Result<Rc<Env>, RuntimeError> {
    let mut env = env;
    for binding in &block.bindings {
        env = define(
            env,
            binding.name.clone(),
            Value::TypeConstructor(binding.name.clone()),
        );
    }
    Ok(env)
}

fn eval_let(block: &LetBlock, env: Rc<Env>) -> Result<(Rc<Env>, Value), RuntimeError> {
    let mut env = env;
    for binding in &block.bindings {
        let value = eval_expr(&binding.value, &env)?;
        env = define(env, binding.name.clone(), value);
    }
    let value = eval_expr(&block.body, &env)?;
    Ok((env, value))
}

fn eval_expr(expr: &Expr, env: &Rc<Env>) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Ident(name) => env
            .get(name)
            .ok_or_else(|| RuntimeError(format!("undefined identifier: {name}"))),
        Expr::Number(text) => text
            .parse::<i64>()
            .map(Value::Number)
            .map_err(|_| RuntimeError(format!("invalid number: {text}"))),
        Expr::Apply { func, arg } => {
            let func = eval_expr(func, env)?;
            let arg = eval_expr(arg, env)?;
            apply(func, arg)
        }
        Expr::Lambda { params, body } => Ok(Value::Closure {
            params: params.clone(),
            body: body.as_ref().clone(),
            env: Rc::clone(env),
        }),
        Expr::Access { target, field, kind } => {
            let target = eval_expr(target, env)?;
            access(target, field, kind)
        }
        Expr::Prefix { expr, .. } => eval_expr(expr, env),
        Expr::Infix { op, left, right } => {
            let left = eval_expr(left, env)?;
            let right = eval_expr(right, env)?;
            infix(*op, left, right)
        }
        Expr::Record(fields) => {
            let mut values = Vec::new();
            for field in fields {
                let value = eval_expr(&field.value, env)?;
                values.push((field.name.clone(), value));
            }
            Ok(Value::Record(values))
        }
    }
}

fn apply(func: Value, arg: Value) -> Result<Value, RuntimeError> {
    match func {
        Value::Closure {
            mut params,
            body,
            env,
        } => {
            let name = params.remove(0);
            let env = define(env, name, arg);
            if params.is_empty() {
                eval_expr(&body, &env)
            } else {
                Ok(Value::Closure {
                    params,
                    body,
                    env,
                })
            }
        }
        Value::TypeConstructor(name) => Ok(Value::TypeInstance(name, Box::new(arg))),
        Value::Builtin(_, func) => func(arg),
        _ => Err(RuntimeError(format!("not a function: {func}"))),
    }
}

fn access(target: Value, field: &str, kind: &AccessKind) -> Result<Value, RuntimeError> {
    match (target, kind) {
        (Value::Record(fields), AccessKind::Value) => fields
            .into_iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value)
            .ok_or_else(|| RuntimeError(format!("missing field: {field}"))),
        (Value::TypeInstance(_, value), AccessKind::Value) => access(*value, field, kind),
        (target, _) => Err(RuntimeError(format!(
            "cannot access {field} on value: {target}"
        ))),
    }
}

fn infix(op: InfixOp, left: Value, right: Value) -> Result<Value, RuntimeError> {
    let result = match (&left, &right) {
        (Value::Number(left), Value::Number(right)) => match op {
            InfixOp::Add => left + right,
            InfixOp::Sub => left - right,
            InfixOp::Mul => left * right,
        },
        _ => {
            return Err(RuntimeError(format!(
                "cannot apply arithmetic to {left} and {right}"
            )))
        }
    };

    Ok(Value::Number(result))
}
