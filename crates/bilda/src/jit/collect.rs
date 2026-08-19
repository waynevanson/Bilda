use crate::ast::{Ast, Call, Expression, LetIn, Map, Math, MathTarget, Product, Sum};
use crate::jit::env::Env;

pub(crate) fn collect_lambdas_ast<'a>(ast: &Ast<'a>, out: &mut Vec<*const Expression<'a>>) {
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

pub(crate) fn collect_lambdas_expr<'a>(expr: &Expression<'a>, out: &mut Vec<*const Expression<'a>>) {
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
        Expression::Not(_) => {}
        _ => {}
    }
}

#[allow(clippy::only_used_in_recursion)]
pub(crate) fn collect_math_target<'a>(target: &MathTarget<'a>, out: &mut Vec<*const Expression<'a>>) {
    if let MathTarget::Math(math) = target {
        collect_math_target(&math.left, out);
        collect_math_target(&math.right, out);
    }
}

pub(crate) fn value_is_function<'a>(ast: &Ast<'a>, env: &Env<'a>) -> bool {
    match ast {
        Ast::Expression(expr) => expr_is_function(expr, env),
        Ast::LetIn(LetIn { expression, .. }) => expr_is_function(expression, env),
    }
}

pub(crate) fn expr_is_function<'a>(expr: &Expression<'a>, env: &Env<'a>) -> bool {
    match expr {
        Expression::Lambda(_) => true,
        Expression::Reference(name) => env.get(name).map(|s| s.is_function).unwrap_or(false),
        _ => false,
    }
}
