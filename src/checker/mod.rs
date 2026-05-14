use crate::ast::*;
use crate::error::{OnewayError, Span};

const BUILTIN_TYPES: &[&str] = &["Noop", "String", "Stdout"];

pub fn check(module: &Module) -> Vec<OnewayError> {
    let mut errors = Vec::new();
    let mut main_found = false;

    for item in &module.items {
        let Item::Function(func) = item;
        check_function(func, &mut errors, &mut main_found);
    }

    if !main_found {
        errors.push(OnewayError::CheckError {
            message: "no `main` entry point defined".to_string(),
            span: module.span,
        });
    }

    errors
}

fn check_function(func: &FunctionDef, errors: &mut Vec<OnewayError>, main_found: &mut bool) {
    if func.name.name == "main" {
        if *main_found {
            errors.push(OnewayError::CheckError {
                message: "duplicate `main` definition".to_string(),
                span: func.span,
            });
        }
        *main_found = true;

        if func.receiver.is_some() {
            errors.push(OnewayError::CheckError {
                message: "`main` is the entry point and must not have a receiver".to_string(),
                span: func.span,
            });
        }
    }

    check_type_known(&func.return_ty, errors);
    for param in &func.params {
        check_type_known(&param.ty, errors);
    }

    let scope = Scope::from_function(func);
    check_block(&func.body, &func.return_ty, &scope, errors);
}

struct Scope {
    names: Vec<String>,
}

impl Scope {
    fn from_function(func: &FunctionDef) -> Self {
        let mut names: Vec<String> = func.params.iter().map(|p| p.ty.name.clone()).collect();
        if let Some(recv) = &func.receiver {
            names.push(recv.name.clone());
        }
        Self { names }
    }

    fn contains(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }
}

fn check_block(
    block: &Block,
    return_ty: &TypeExpr,
    scope: &Scope,
    errors: &mut Vec<OnewayError>,
) {
    if block.exprs.is_empty() {
        errors.push(OnewayError::CheckError {
            message: "function body must contain at least one expression".to_string(),
            span: block.span,
        });
        return;
    }

    for expr in &block.exprs {
        check_expr(expr, scope, errors);
    }

    let last = block.exprs.last().unwrap();
    let last_ty = expr_type_name(last);
    if last_ty != return_ty.name {
        errors.push(OnewayError::CheckError {
            message: format!(
                "function returns `{}` but last expression has type `{}`",
                return_ty.name, last_ty
            ),
            span: last.span(),
        });
    }
}

fn check_expr(expr: &Expr, scope: &Scope, errors: &mut Vec<OnewayError>) {
    match expr {
        Expr::Ident(ident) => {
            if !BUILTIN_TYPES.contains(&ident.name.as_str()) && !scope.contains(&ident.name) {
                errors.push(OnewayError::CheckError {
                    message: format!("unknown name `{}`", ident.name),
                    span: ident.span,
                });
            }
        }
        Expr::StringLit { .. } => {}
        Expr::MethodCall {
            receiver,
            method,
            args,
            span,
        } => {
            check_expr(receiver, scope, errors);
            for arg in args {
                check_expr(arg, scope, errors);
            }
            let recv_ty = expr_type_name(receiver);
            if !is_known_method(&recv_ty, &method.name, args.len()) {
                errors.push(OnewayError::CheckError {
                    message: format!(
                        "no method `{}` on type `{}` with {} argument(s)",
                        method.name,
                        recv_ty,
                        args.len()
                    ),
                    span: *span,
                });
            }
        }
    }
}

fn is_known_method(receiver_ty: &str, method: &str, arg_count: usize) -> bool {
    matches!(
        (receiver_ty, method, arg_count),
        ("String", "print", 1)
    )
}

fn check_type_known(ty: &TypeExpr, errors: &mut Vec<OnewayError>) {
    if !BUILTIN_TYPES.contains(&ty.name.as_str()) {
        errors.push(OnewayError::CheckError {
            message: format!("unknown type `{}`", ty.name),
            span: ty.span,
        });
    }
}

fn expr_type_name(expr: &Expr) -> String {
    match expr {
        Expr::Ident(ident) => ident.name.clone(),
        Expr::StringLit { .. } => "String".to_string(),
        Expr::MethodCall {
            receiver, method, ..
        } => {
            let recv_ty = expr_type_name(receiver);
            method_return_type(&recv_ty, &method.name)
        }
    }
}

fn method_return_type(receiver_ty: &str, method: &str) -> String {
    match (receiver_ty, method) {
        ("String", "print") => "Noop".to_string(),
        _ => "<unknown>".to_string(),
    }
}

#[allow(dead_code)]
fn _expr_span(expr: &Expr) -> Span {
    expr.span()
}
