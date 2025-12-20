use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::system::tree_sitter_adapter::TreeSitterAdapter;

use super::directives::context_from_directive;
use super::types::{CallSite, CallTarget, ParsedDecl, ParsedModuleData};
use super::utils::normalize_union_parts;

pub(crate) fn parse_bsl_module_ast_with_progress(
    tree: &tree_sitter::Tree,
    source: &str,
    mut progress: impl FnMut(usize, usize),
) -> Result<ParsedModuleData> {
    let parse_result =
        TreeSitterAdapter::convert_tree_fast_with_progress(tree, source, &mut progress)
            .map_err(|e| anyhow!("tree-sitter convert_tree_fast failed: {}", e))?;
    let (decls, call_sites) = collect_decls_and_call_sites(&parse_result.program.statements);
    Ok(ParsedModuleData { decls, call_sites })
}

pub(crate) fn collect_decls_and_call_sites(
    statements: &[crate::parsing::bsl::ast::Statement],
) -> (Vec<ParsedDecl>, Vec<CallSite>) {
    use crate::parsing::bsl::ast::{Expression, Statement};

    type VarEnv = HashMap<String, Vec<String>>;

    fn env_get_union(env: &VarEnv, name: &str) -> Option<String> {
        let mut parts = env.get(name)?.clone();
        parts = normalize_union_parts(parts);
        (!parts.is_empty()).then(|| parts.join(" | "))
    }

    fn env_add_type(env: &mut VarEnv, name: &str, t: String) {
        env.entry(name.to_string()).or_default().push(t);
        if let Some(v) = env.get_mut(name) {
            *v = normalize_union_parts(std::mem::take(v));
        }
    }

    fn expr_to_dotted_path(expr: &Expression) -> Option<Vec<String>> {
        match expr {
            Expression::Identifier { name, .. } => Some(vec![name.clone()]),
            Expression::PropertyAccess { object, property, .. } => {
                let mut base = expr_to_dotted_path(object)?;
                base.push(property.clone());
                Some(base)
            }
            _ => None,
        }
    }

    fn infer_expr_type_with_env(expr: &Expression, env: &VarEnv) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => env_get_union(env, name).or_else(|| {
                name.eq_ignore_ascii_case("неопределено")
                    .then(|| "Неопределено".to_string())
            }),
            _ => infer_expr_type(expr),
        }
    }

    fn walk_expr(acc: &mut Vec<CallSite>, expr: &Expression, env: &VarEnv) {
        match expr {
            Expression::Call { function, args, .. } => {
                match function.as_ref() {
                    Expression::Identifier { name, .. } => acc.push(CallSite {
                        target: CallTarget::LocalFunction { name: name.clone() },
                        arg_types: args
                            .iter()
                            .map(|a| infer_expr_type_with_env(a, env))
                            .collect(),
                    }),
                    Expression::PropertyAccess {
                        object,
                        property,
                        ..
                    } => {
                        if let Some(receiver) = expr_to_dotted_path(object) {
                            acc.push(CallSite {
                                target: CallTarget::QualifiedMethod {
                                    receiver,
                                    name: property.clone(),
                                },
                                arg_types: args
                                    .iter()
                                    .map(|a| infer_expr_type_with_env(a, env))
                                    .collect(),
                            });
                        }
                    }
                    _ => {}
                }

                walk_expr(acc, function, env);
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::Binary { left, right, .. } => {
                walk_expr(acc, left, env);
                walk_expr(acc, right, env);
            }
            Expression::Unary { operand, .. } => walk_expr(acc, operand, env),
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                walk_expr(acc, condition, env);
                walk_expr(acc, then_expr, env);
                walk_expr(acc, else_expr, env);
            }
            Expression::New { args, .. } => {
                for a in args {
                    walk_expr(acc, a, env);
                }
            }
            Expression::PropertyAccess { object, .. } => walk_expr(acc, object, env),
            Expression::IndexAccess { object, index, .. } => {
                walk_expr(acc, object, env);
                walk_expr(acc, index, env);
            }
            Expression::Await { expression, .. } => walk_expr(acc, expression, env),
            Expression::Identifier { .. }
            | Expression::String { .. }
            | Expression::Number { .. }
            | Expression::Boolean { .. }
            | Expression::Date { .. } => {}
        }
    }

    fn merge_envs(a: &mut VarEnv, b: VarEnv) {
        for (k, v) in b {
            for t in v {
                env_add_type(a, &k, t);
            }
        }
    }

    fn walk_block(
        decls: &mut Vec<ParsedDecl>,
        calls: &mut Vec<CallSite>,
        statements: &[Statement],
        env: &mut VarEnv,
    ) {
        for st in statements {
            walk_stmt(decls, calls, st, env);
        }
    }

    fn walk_stmt(
        decls: &mut Vec<ParsedDecl>,
        calls: &mut Vec<CallSite>,
        st: &Statement,
        env: &mut VarEnv,
    ) {
        match st {
            Statement::Assignment { target, value, .. } => {
                walk_expr(calls, target, env);
                walk_expr(calls, value, env);

                if let Expression::Identifier { name, .. } = target {
                    if let Some(t) = infer_expr_type_with_env(value, env) {
                        env_add_type(env, name, t);
                    }
                }
            }
            Statement::VarDeclaration { name, type_hint, .. } => {
                if let Some(hint) = type_hint.as_ref().filter(|s| !s.trim().is_empty()) {
                    env_add_type(env, name, hint.trim().to_string());
                }
            }
            Statement::FunctionDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export,
                span,
                ..
            } => {
                let params = params
                    .iter()
                    .map(|name| super::types::ParsedParam {
                        name: name.clone(),
                        is_optional: false,
                    })
                    .collect();
                decls.push(ParsedDecl {
                    return_type: infer_return_type_from_body(body),
                    name: name.clone(),
                    params,
                    is_export: *is_export,
                    directive_ctx: compiler_directive.map(context_from_directive),
                    span: *span,
                });
                let mut nested_env = env.clone();
                walk_block(decls, calls, body, &mut nested_env);
            }
            Statement::ProcedureDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export,
                span,
                ..
            } => {
                let params = params
                    .iter()
                    .map(|name| super::types::ParsedParam {
                        name: name.clone(),
                        is_optional: false,
                    })
                    .collect();
                decls.push(ParsedDecl {
                    return_type: None,
                    name: name.clone(),
                    params,
                    is_export: *is_export,
                    directive_ctx: compiler_directive.map(context_from_directive),
                    span: *span,
                });
                let mut nested_env = env.clone();
                walk_block(decls, calls, body, &mut nested_env);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(calls, condition, env);

                let mut then_env = env.clone();
                walk_block(decls, calls, then_body, &mut then_env);

                let mut else_env = env.clone();
                if let Some(else_body) = else_body {
                    walk_block(decls, calls, else_body, &mut else_env);
                }

                *env = HashMap::new();
                merge_envs(env, then_env);
                merge_envs(env, else_env);
            }
            Statement::For {
                start, end, body, ..
            } => {
                walk_expr(calls, start, env);
                walk_expr(calls, end, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::ForEach { collection, body, .. } => {
                walk_expr(calls, collection, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::While { condition, body, .. } => {
                walk_expr(calls, condition, env);

                let mut body_env = env.clone();
                walk_block(decls, calls, body, &mut body_env);
                merge_envs(env, body_env);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    walk_expr(calls, v, env);
                }
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                let mut try_env = env.clone();
                walk_block(decls, calls, try_body, &mut try_env);

                let mut except_env = env.clone();
                walk_block(decls, calls, except_body, &mut except_env);

                *env = HashMap::new();
                merge_envs(env, try_env);
                merge_envs(env, except_env);
            }
            Statement::Call { expression, .. } => walk_expr(calls, expression, env),
            Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Goto { .. }
            | Statement::Label { .. } => {}
            Statement::Execute { code, .. } => walk_expr(calls, code, env),
            Statement::RaiseError { message, .. } => {
                if let Some(m) = message {
                    walk_expr(calls, m, env);
                }
            }
            Statement::AddHandler {
                event, handler, ..
            }
            | Statement::RemoveHandler {
                event, handler, ..
            } => {
                walk_expr(calls, event, env);
                walk_expr(calls, handler, env);
            }
            Statement::Await { expression, .. } => walk_expr(calls, expression, env),
        }
    }

    let mut decls = Vec::new();
    let mut calls = Vec::new();
    let mut env: VarEnv = HashMap::new();
    walk_block(&mut decls, &mut calls, statements, &mut env);
    (decls, calls)
}

pub(crate) fn infer_return_type_from_body(
    body: &[crate::parsing::bsl::ast::Statement],
) -> Option<String> {
    use crate::parsing::bsl::ast::Statement;

    fn collect_return_types(acc: &mut Vec<String>, statements: &[Statement]) {
        for st in statements {
            match st {
                Statement::Return { value, .. } => {
                    let inferred = match value {
                        None => Some("Неопределено".to_string()),
                        Some(expr) => infer_expr_type(expr),
                    };
                    if let Some(t) = inferred {
                        acc.push(t);
                    }
                }
                Statement::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    collect_return_types(acc, then_body);
                    if let Some(else_body) = else_body {
                        collect_return_types(acc, else_body);
                    }
                }
                Statement::For { body, .. }
                | Statement::ForEach { body, .. }
                | Statement::While { body, .. } => {
                    collect_return_types(acc, body);
                }
                Statement::Try {
                    try_body,
                    except_body,
                    ..
                } => {
                    collect_return_types(acc, try_body);
                    collect_return_types(acc, except_body);
                }
                _ => {}
            }
        }
    }

    let mut collected = Vec::new();
    collect_return_types(&mut collected, body);

    if collected.is_empty() {
        return Some("Неопределено".to_string());
    }

    Some(normalize_union_parts(collected).join(" | "))
}

pub(crate) fn infer_expr_type(
    expr: &crate::parsing::bsl::ast::Expression,
) -> Option<String> {
    use crate::parsing::bsl::ast::Expression;
    match expr {
        Expression::String { .. } => Some("Строка".to_string()),
        Expression::Number { .. } => Some("Число".to_string()),
        Expression::Boolean { .. } => Some("Булево".to_string()),
        Expression::Date { .. } => Some("Дата".to_string()),
        Expression::New { type_name, .. } => Some(type_name.clone()),
        Expression::Identifier { name, .. } if name.eq_ignore_ascii_case("неопределено") => {
            Some("Неопределено".to_string())
        }
        _ => None,
    }
}
