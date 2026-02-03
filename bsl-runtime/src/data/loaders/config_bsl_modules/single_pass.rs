use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::system::tree_sitter_adapter::directives::find_preceding_directive;
use crate::system::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};
use crate::system::tree_sitter_adapter::utils::node_text;

use super::directives::context_from_directive;
use super::types::{
    CallSite, CallTarget, ParsedDecl, ParsedModuleData, ReturnAtom, ReturnFacts, SinglePassMode,
};
use super::utils::normalize_union_parts;

pub(crate) fn parse_bsl_module_tree_sitter_with_mode(
    tree: &tree_sitter::Tree,
    source: &str,
    mode: SinglePassMode,
) -> Result<ParsedModuleData> {
    let line_index = LineIndex::new(source);
    let mut decls: Vec<ParsedDecl> = Vec::new();
    let mut call_sites: Vec<CallSite> = Vec::new();
    let mut env: HashMap<String, Vec<String>> = HashMap::new();
    let mut return_types: Vec<String> = Vec::new();
    let mut env_atoms: HashMap<String, Vec<ReturnAtom>> = HashMap::new();
    let mut return_atoms: Vec<ReturnAtom> = Vec::new();
    let mut has_return_without_value = false;
    let track_returns = matches!(mode, SinglePassMode::Full);

    let mut cursor = tree.root_node().walk();
    for child in tree.root_node().children(&mut cursor) {
        walk_stmt_ts(
            &child,
            source,
            &line_index,
            &mut decls,
            &mut call_sites,
            &mut env,
            &mut return_types,
            &mut env_atoms,
            &mut return_atoms,
            &mut has_return_without_value,
            track_returns,
        );
    }

    Ok(ParsedModuleData { decls, call_sites })
}

#[allow(clippy::too_many_arguments)]
fn walk_stmt_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    env_atoms: &mut HashMap<String, Vec<ReturnAtom>>,
    return_atoms: &mut Vec<ReturnAtom>,
    has_return_without_value: &mut bool,
    track_returns: bool,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(mut decl) = parse_decl_ts(node, source, line_index) {
                let mut nested_env = env.clone();
                let mut nested_env_atoms = seed_env_atoms_from_types(&nested_env);
                let mut func_return_types = Vec::new();
                let mut func_return_atoms: Vec<ReturnAtom> = Vec::new();
                let mut func_has_return_without_value = false;
                walk_function_body_ts(
                    node,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut nested_env,
                    &mut nested_env_atoms,
                    &mut func_return_types,
                    &mut func_return_atoms,
                    &mut func_has_return_without_value,
                    track_returns,
                );
                if track_returns {
                    decl.return_type = finalize_return_types(func_return_types);
                    decl.return_facts = Some(finalize_return_facts(
                        func_return_atoms,
                        nested_env_atoms,
                        func_has_return_without_value,
                    ));
                } else {
                    decl.return_type = None;
                    decl.return_facts = None;
                }
                decls.push(decl);
            }
        }
        "procedure_definition" => {
            if let Some(mut decl) = parse_decl_ts(node, source, line_index) {
                let mut nested_env = env.clone();
                let mut nested_env_atoms = seed_env_atoms_from_types(&nested_env);
                let mut func_return_types = Vec::new();
                let mut proc_return_atoms: Vec<ReturnAtom> = Vec::new();
                let mut proc_has_return_without_value = false;
                walk_function_body_ts(
                    node,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut nested_env,
                    &mut nested_env_atoms,
                    &mut func_return_types,
                    &mut proc_return_atoms,
                    &mut proc_has_return_without_value,
                    false,
                );
                decl.return_type = None;
                decl.return_facts = None;
                decls.push(decl);
            }
        }
        "assignment_statement" => {
            if let Some((target, value_node)) = split_assignment_ts(node, source) {
                walk_expr_ts(&value_node, source, calls, env);
                if let Some(atom) = infer_expr_atom_ts(&value_node, source) {
                    env_atoms.entry(target.clone()).or_default().push(atom);
                    normalize_env_atoms_var(env_atoms, &target);
                }
                if let Some(t) = infer_expr_type_ts(&value_node, source, env) {
                    env.entry(target.clone()).or_default().push(t);
                    if let Some(v) = env.get_mut(&target) {
                        *v = normalize_union_parts(std::mem::take(v));
                    }
                }
            }
        }
        "var_definition" | "var_statement" => {
            if let Some(name) = first_identifier(node, source) {
                env.entry(name.clone()).or_default();
                env_atoms.entry(name).or_default();
            }
        }
        "call_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression"
                    || child.kind() == "call_expression"
                    || child.kind() == "method_call"
                {
                    walk_expr_ts(&child, source, calls, env);
                }
            }
        }
        "return_statement" => {
            let mut cursor = node.walk();
            let mut found_expr = false;
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    found_expr = true;
                    if track_returns {
                        if let Some(t) = infer_expr_type_ts(&child, source, env) {
                            return_types.push(t);
                        }
                        return_atoms.push(
                            infer_expr_atom_ts(&child, source).unwrap_or(ReturnAtom::Unknown),
                        );
                    }
                    walk_expr_ts(&child, source, calls, env);
                }
            }
            if !found_expr && track_returns {
                return_types.push("Неопределено".to_string());
                *has_return_without_value = true;
            }
        }
        "if_statement" => {
            walk_if_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                env_atoms,
                return_atoms,
                has_return_without_value,
                track_returns,
            );
        }
        "for_statement" | "for_each_statement" | "while_statement" => {
            walk_loop_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                env_atoms,
                return_atoms,
                has_return_without_value,
                track_returns,
            );
        }
        "try_statement" => {
            walk_try_ts(
                node,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                env_atoms,
                return_atoms,
                has_return_without_value,
                track_returns,
            );
        }
        "execute_statement" | "raise_error_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    walk_expr_ts(&child, source, calls, env);
                }
            }
        }
        "add_handler_statement" | "remove_handler_statement" | "await_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "expression" || child.kind() == "method_call" {
                    walk_expr_ts(&child, source, calls, env);
                }
            }
        }
        _ => {}
    }
}

fn parse_decl_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
) -> Option<ParsedDecl> {
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params: Vec<super::types::ParsedParam> = Vec::new();
    let mut is_export = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" if name.is_empty() => name = node_text(&child, source),
            "parameters" => {
                params = parse_parameters_ts(&child, source);
            }
            _ if child.kind().ends_with("_KEYWORD") => {
                let kw = node_text(&child, source).trim().to_lowercase();
                if kw == "экспорт" || kw == "export" {
                    is_export = true;
                }
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return None;
    }

    let directive_ctx = find_preceding_directive(node, source).map(context_from_directive);

    Some(ParsedDecl {
        name,
        params,
        is_export,
        directive_ctx,
        return_type: None,
        return_facts: None,
        span: node_to_span_cached(node, source, line_index),
    })
}

#[allow(clippy::too_many_arguments)]
fn walk_function_body_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    env_atoms: &mut HashMap<String, Vec<ReturnAtom>>,
    return_types: &mut Vec<String>,
    return_atoms: &mut Vec<ReturnAtom>,
    has_return_without_value: &mut bool,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().ends_with("_statement") || child.kind().ends_with("_definition") {
            walk_stmt_ts(
                &child,
                source,
                line_index,
                decls,
                calls,
                env,
                return_types,
                env_atoms,
                return_atoms,
                has_return_without_value,
                track_returns,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_if_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    env_atoms: &mut HashMap<String, Vec<ReturnAtom>>,
    return_atoms: &mut Vec<ReturnAtom>,
    has_return_without_value: &mut bool,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut in_then = false;
    let mut then_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut else_nodes: Vec<tree_sitter::Node> = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "THEN_KEYWORD" | "ТОГДА_KEYWORD" => in_then = true,
            "ENDIF_KEYWORD" | "КОНЕЦЕСЛИ_KEYWORD" => break,
            _ if !in_then
                && (child.kind().contains("expression")
                    || child.kind() == "call_expression"
                    || child.kind() == "method_call") =>
            {
                walk_expr_ts(&child, source, calls, env);
            }
            "else_clause" | "elseif_clause" => else_nodes.push(child),
            kind if in_then && (kind.ends_with("_statement") || kind.ends_with("_definition")) => {
                then_nodes.push(child);
            }
            _ => {}
        }
    }

    let mut then_env = env.clone();
    let mut then_env_atoms = env_atoms.clone();
    for node in then_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut then_env,
            return_types,
            &mut then_env_atoms,
            return_atoms,
            has_return_without_value,
            track_returns,
        );
    }

    let mut else_env = env.clone();
    let mut else_env_atoms = env_atoms.clone();
    for node in else_nodes {
        let mut clause_cursor = node.walk();
        for child in node.children(&mut clause_cursor) {
            if child.kind().ends_with("_statement") || child.kind().ends_with("_definition") {
                walk_stmt_ts(
                    &child,
                    source,
                    line_index,
                    decls,
                    calls,
                    &mut else_env,
                    return_types,
                    &mut else_env_atoms,
                    return_atoms,
                    has_return_without_value,
                    track_returns,
                );
            }
        }
    }

    *env = HashMap::new();
    merge_envs(env, then_env);
    merge_envs(env, else_env);

    *env_atoms = HashMap::new();
    merge_env_atoms(env_atoms, then_env_atoms);
    merge_env_atoms(env_atoms, else_env_atoms);
}

#[allow(clippy::too_many_arguments)]
fn walk_loop_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    env_atoms: &mut HashMap<String, Vec<ReturnAtom>>,
    return_atoms: &mut Vec<ReturnAtom>,
    has_return_without_value: &mut bool,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut body_nodes: Vec<tree_sitter::Node> = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            _ if child.kind().contains("expression")
                || child.kind() == "call_expression"
                || child.kind() == "method_call" =>
            {
                walk_expr_ts(&child, source, calls, env);
            }
            kind if kind.ends_with("_statement") || kind.ends_with("_definition") => {
                body_nodes.push(child);
            }
            _ => {}
        }
    }

    let mut body_env = env.clone();
    let mut body_env_atoms = env_atoms.clone();
    for node in body_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut body_env,
            return_types,
            &mut body_env_atoms,
            return_atoms,
            has_return_without_value,
            track_returns,
        );
    }

    merge_envs(env, body_env);
    merge_env_atoms(env_atoms, body_env_atoms);
}

#[allow(clippy::too_many_arguments)]
fn walk_try_ts(
    node: &tree_sitter::Node,
    source: &str,
    line_index: &LineIndex,
    decls: &mut Vec<ParsedDecl>,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
    return_types: &mut Vec<String>,
    env_atoms: &mut HashMap<String, Vec<ReturnAtom>>,
    return_atoms: &mut Vec<ReturnAtom>,
    has_return_without_value: &mut bool,
    track_returns: bool,
) {
    let mut cursor = node.walk();
    let mut try_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut except_nodes: Vec<tree_sitter::Node> = Vec::new();
    let mut in_except = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => in_except = true,
            kind if kind.ends_with("_statement") || kind.ends_with("_definition") => {
                if in_except {
                    except_nodes.push(child);
                } else {
                    try_nodes.push(child);
                }
            }
            _ => {}
        }
    }

    let mut try_env = env.clone();
    let mut try_env_atoms = env_atoms.clone();
    for node in try_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut try_env,
            return_types,
            &mut try_env_atoms,
            return_atoms,
            has_return_without_value,
            track_returns,
        );
    }

    let mut except_env = env.clone();
    let mut except_env_atoms = env_atoms.clone();
    for node in except_nodes {
        walk_stmt_ts(
            &node,
            source,
            line_index,
            decls,
            calls,
            &mut except_env,
            return_types,
            &mut except_env_atoms,
            return_atoms,
            has_return_without_value,
            track_returns,
        );
    }

    *env = HashMap::new();
    merge_envs(env, try_env);
    merge_envs(env, except_env);

    *env_atoms = HashMap::new();
    merge_env_atoms(env_atoms, try_env_atoms);
    merge_env_atoms(env_atoms, except_env_atoms);
}

fn walk_expr_ts(
    node: &tree_sitter::Node,
    source: &str,
    calls: &mut Vec<CallSite>,
    env: &mut HashMap<String, Vec<String>>,
) {
    if node.kind() == "method_call" || node.kind() == "call_expression" {
        if let Some(call) = parse_call_ts(node, source, env) {
            calls.push(call);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "arguments" {
                let mut args_cursor = child.walk();
                for arg_child in child.children(&mut args_cursor) {
                    if arg_child.kind() == "expression" {
                        walk_expr_ts(&arg_child, source, calls, env);
                    }
                }
            }
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_expr_ts(&child, source, calls, env);
    }
}

fn parse_parameters_ts(node: &tree_sitter::Node, source: &str) -> Vec<super::types::ParsedParam> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() != "parameter" {
            continue;
        }

        let mut name: Option<String> = None;
        let mut is_optional = false;
        let mut param_cursor = child.walk();
        for param_child in child.children(&mut param_cursor) {
            if param_child.kind() == "identifier" && name.is_none() {
                name = Some(node_text(&param_child, source));
                continue;
            }

            if param_child.kind() == "=" {
                is_optional = true;
            }
        }

        if !is_optional {
            let param_text = node_text(&child, source);
            if param_text.contains('=') {
                is_optional = true;
            }
        }

        if let Some(name) = name {
            params.push(super::types::ParsedParam { name, is_optional });
        }
    }

    params
}

fn parse_call_ts(
    node: &tree_sitter::Node,
    source: &str,
    env: &HashMap<String, Vec<String>>,
) -> Option<CallSite> {
    let mut cursor = node.walk();
    let mut access_node: Option<tree_sitter::Node> = None;
    let mut method_call_node: Option<tree_sitter::Node> = None;
    let mut func_name: Option<String> = None;
    let mut args: Vec<tree_sitter::Node> = Vec::new();

    if node.kind() == "method_call" {
        method_call_node = Some(*node);
    }

    for child in node.children(&mut cursor) {
        match child.kind() {
            "access" => access_node = Some(child),
            "method_call" => method_call_node = Some(child),
            "identifier" if func_name.is_none() => {
                func_name = Some(node_text(&child, source));
            }
            "arguments" => {
                let mut args_cursor = child.walk();
                for arg_child in child.children(&mut args_cursor) {
                    if arg_child.kind() == "expression" {
                        args.push(arg_child);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(method_call) = method_call_node {
        let mut method_cursor = method_call.walk();
        for child in method_call.children(&mut method_cursor) {
            match child.kind() {
                "identifier" if func_name.is_none() => {
                    func_name = Some(node_text(&child, source));
                }
                "arguments" if args.is_empty() => {
                    let mut args_cursor = child.walk();
                    for arg_child in child.children(&mut args_cursor) {
                        if arg_child.kind() == "expression" {
                            args.push(arg_child);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let arg_types = args
        .iter()
        .map(|arg| infer_expr_type_ts(arg, source, env))
        .collect();

    if let (Some(access), Some(method_call)) = (access_node, method_call_node) {
        let receiver = split_dotted_path(&access, source);
        let name = first_identifier(&method_call, source)?;
        return Some(CallSite {
            target: CallTarget::QualifiedMethod { receiver, name },
            arg_types,
        });
    }

    if func_name.is_none() {
        if let Some(access) = access_node {
            func_name = first_identifier(&access, source);
        }
    }

    if let Some(name) = func_name {
        return Some(CallSite {
            target: CallTarget::LocalFunction { name },
            arg_types,
        });
    }

    None
}

fn split_assignment_ts<'a>(
    node: &'a tree_sitter::Node<'a>,
    source: &str,
) -> Option<(String, tree_sitter::Node<'a>)> {
    let mut cursor = node.walk();
    let mut seen_eq = false;
    let mut target_name: Option<String> = None;
    let mut value_expr: Option<tree_sitter::Node> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "=" => seen_eq = true,
            "identifier" if !seen_eq && target_name.is_none() => {
                target_name = Some(node_text(&child, source));
            }
            "expression" | "call_expression" | "const_expression" | "method_call"
                if seen_eq && value_expr.is_none() =>
            {
                value_expr = Some(child);
            }
            _ => {}
        }
    }

    let target = target_name?;
    let value = value_expr?;
    Some((target, value))
}

fn infer_expr_type_ts(
    node: &tree_sitter::Node,
    source: &str,
    env: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if node.kind() == "expression" || node.kind() == "const_expression" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(t) = infer_expr_type_ts(&child, source, env) {
                return Some(t);
            }
        }
        return None;
    }

    match node.kind() {
        "string" => Some("Строка".to_string()),
        "number" => Some("Число".to_string()),
        "date" | "date_literal" => Some("Дата".to_string()),
        "identifier" => {
            let name = node_text(node, source);
            if name.eq_ignore_ascii_case("неопределено") {
                return Some("Неопределено".to_string());
            }
            env_get_union_ts(env, &name)
        }
        "new_expression" | "new_expression_method" => extract_new_type_ts(node, source),
        _ => {
            let text = node_text(node, source).to_lowercase();
            if text == "истина" || text == "true" || text == "ложь" || text == "false" {
                Some("Булево".to_string())
            } else {
                None
            }
        }
    }
}

fn infer_expr_atom_ts(node: &tree_sitter::Node, source: &str) -> Option<ReturnAtom> {
    if node.kind() == "expression" || node.kind() == "const_expression" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(atom) = infer_expr_atom_ts(&child, source) {
                return Some(atom);
            }
        }
        return None;
    }

    match node.kind() {
        "string" => Some(ReturnAtom::Known("Строка".to_string())),
        "number" => Some(ReturnAtom::Known("Число".to_string())),
        "date" | "date_literal" => Some(ReturnAtom::Known("Дата".to_string())),
        "identifier" => {
            let name = node_text(node, source);
            if name.eq_ignore_ascii_case("неопределено") {
                return Some(ReturnAtom::Known("Неопределено".to_string()));
            }
            let lowered = name.to_lowercase();
            if lowered == "истина" || lowered == "true" || lowered == "ложь" || lowered == "false"
            {
                return Some(ReturnAtom::Known("Булево".to_string()));
            }
            Some(ReturnAtom::Var(name))
        }
        "new_expression" | "new_expression_method" => {
            extract_new_type_ts(node, source).map(ReturnAtom::Known)
        }
        "method_call" | "call_expression" => {
            parse_call_target_ts(node, source).map(ReturnAtom::Call)
        }
        _ => None,
    }
}

fn parse_call_target_ts(node: &tree_sitter::Node, source: &str) -> Option<CallTarget> {
    let mut cursor = node.walk();
    let mut access_node: Option<tree_sitter::Node> = None;
    let mut method_call_node: Option<tree_sitter::Node> = None;
    let mut func_name: Option<String> = None;

    if node.kind() == "method_call" {
        method_call_node = Some(*node);
    }

    for child in node.children(&mut cursor) {
        match child.kind() {
            "access" => access_node = Some(child),
            "method_call" => method_call_node = Some(child),
            "identifier" if func_name.is_none() => {
                func_name = Some(node_text(&child, source));
            }
            _ => {}
        }
    }

    if let Some(method_call) = method_call_node {
        let mut method_cursor = method_call.walk();
        for child in method_call.children(&mut method_cursor) {
            if child.kind() == "identifier" && func_name.is_none() {
                func_name = Some(node_text(&child, source));
            }
        }
    }

    if let (Some(access), Some(method_call)) = (access_node, method_call_node) {
        let receiver = split_dotted_path(&access, source);
        let name = first_identifier(&method_call, source)?;
        // Tree-sitter иногда отдаёт локальный вызов `Функция()` в форме `method_call/access`.
        // В этом случае receiver и name совпадают (receiver = ["Функция"], name = "Функция").
        if receiver.len() == 1 && receiver[0].eq_ignore_ascii_case(&name) {
            return Some(CallTarget::LocalFunction { name });
        }
        return Some(CallTarget::QualifiedMethod { receiver, name });
    }

    if func_name.is_none() {
        if let Some(access) = access_node {
            func_name = first_identifier(&access, source);
        }
    }

    func_name.map(|name| CallTarget::LocalFunction { name })
}

fn extract_new_type_ts(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut type_expr: Option<String> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "NEW_KEYWORD" | "НОВЫЙ_KEYWORD" => {}
            "identifier" | "property_access" => {
                if type_expr.is_none() {
                    type_expr = Some(node_text(&child, source));
                }
            }
            "arguments" => {
                let mut arg_cursor = child.walk();
                for arg_child in child.children(&mut arg_cursor) {
                    if arg_child.kind() == "expression" && type_expr.is_none() {
                        type_expr = Some(node_text(&arg_child, source));
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    type_expr
}

fn split_dotted_path(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let text = node_text(node, source);
    text.split('.')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

fn first_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(node_text(&child, source));
        }
    }
    None
}

fn finalize_return_types(collected: Vec<String>) -> Option<String> {
    if collected.is_empty() {
        return Some("Неопределено".to_string());
    }

    Some(normalize_union_parts(collected).join(" | "))
}

fn env_get_union_ts(env: &HashMap<String, Vec<String>>, name: &str) -> Option<String> {
    let mut parts = env.get(name)?.clone();
    parts = normalize_union_parts(parts);
    (!parts.is_empty()).then(|| parts.join(" | "))
}

fn merge_envs(target: &mut HashMap<String, Vec<String>>, source: HashMap<String, Vec<String>>) {
    for (k, v) in source {
        for t in v {
            target.entry(k.clone()).or_default().push(t);
        }
        if let Some(values) = target.get_mut(&k) {
            *values = normalize_union_parts(std::mem::take(values));
        }
    }
}

fn seed_env_atoms_from_types(
    env: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<ReturnAtom>> {
    let mut out: HashMap<String, Vec<ReturnAtom>> = HashMap::new();
    for (name, types) in env {
        let atoms = types
            .iter()
            .cloned()
            .map(ReturnAtom::Known)
            .collect::<Vec<_>>();
        out.insert(name.clone(), normalize_atoms(atoms));
    }
    out
}

fn atom_key(atom: &ReturnAtom) -> String {
    match atom {
        ReturnAtom::Known(t) => format!("K:{}", t),
        ReturnAtom::Var(v) => format!("V:{}", v),
        ReturnAtom::Call(CallTarget::LocalFunction { name }) => format!("C:L:{}", name),
        ReturnAtom::Call(CallTarget::QualifiedMethod { receiver, name }) => {
            format!("C:Q:{}:{}", receiver.join("."), name)
        }
        ReturnAtom::Unknown => "U".to_string(),
    }
}

fn normalize_atoms(atoms: Vec<ReturnAtom>) -> Vec<ReturnAtom> {
    let mut by_key: HashMap<String, ReturnAtom> = HashMap::new();
    for atom in atoms {
        by_key.entry(atom_key(&atom)).or_insert(atom);
    }
    let mut keys: Vec<String> = by_key.keys().cloned().collect();
    keys.sort();
    keys.into_iter().filter_map(|k| by_key.remove(&k)).collect()
}

fn normalize_env_atoms_var(env_atoms: &mut HashMap<String, Vec<ReturnAtom>>, name: &str) {
    if let Some(values) = env_atoms.get_mut(name) {
        *values = normalize_atoms(std::mem::take(values));
    }
}

fn merge_env_atoms(
    target: &mut HashMap<String, Vec<ReturnAtom>>,
    source: HashMap<String, Vec<ReturnAtom>>,
) {
    for (k, v) in source {
        for atom in v {
            target.entry(k.clone()).or_default().push(atom);
        }
        normalize_env_atoms_var(target, &k);
    }
}

fn finalize_return_facts(
    returns: Vec<ReturnAtom>,
    env_atoms: HashMap<String, Vec<ReturnAtom>>,
    has_return_without_value: bool,
) -> ReturnFacts {
    let returns = normalize_atoms(returns);

    let mut referenced_vars: HashSet<String> = HashSet::new();
    for atom in &returns {
        if let ReturnAtom::Var(name) = atom {
            referenced_vars.insert(name.clone());
        }
    }

    let mut vars: HashMap<String, Vec<ReturnAtom>> = HashMap::new();
    for name in referenced_vars {
        if let Some(values) = env_atoms.get(&name) {
            vars.insert(name, values.clone());
        }
    }

    let has_dynamic = returns.iter().any(|a| matches!(a, ReturnAtom::Unknown))
        || vars
            .values()
            .any(|v| v.iter().any(|a| matches!(a, ReturnAtom::Unknown)));

    ReturnFacts {
        returns,
        vars,
        has_return_without_value,
        has_dynamic,
    }
}

#[allow(dead_code)]
fn extract_export_decls_from_tree(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Result<Vec<ParsedDecl>> {
    fn node_text(node: &tree_sitter::Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    fn convert_parameters(
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<super::types::ParsedParam> {
        let mut out = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                out.push(super::types::ParsedParam {
                    name: node_text(&child, source),
                    is_optional: false,
                });
            }
        }
        out
    }

    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    let mut cursor = tree.root_node().walk();
    for node in tree.root_node().children(&mut cursor) {
        if node.kind() != "function_definition" && node.kind() != "procedure_definition" {
            continue;
        }
        let mut name = String::new();
        let mut params: Vec<super::types::ParsedParam> = Vec::new();
        let mut is_export = false;

        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            match child.kind() {
                "identifier" if name.is_empty() => name = node_text(&child, source),
                "parameters" => params = convert_parameters(&child, source),
                _ if child.kind().ends_with("_KEYWORD") => {
                    let kw = node_text(&child, source).trim().to_lowercase();
                    if kw == "экспорт" || kw == "export" {
                        is_export = true;
                    }
                }
                _ => {}
            }
        }

        if name.is_empty() {
            continue;
        }

        let directive_ctx = find_preceding_directive(&node, source).map(context_from_directive);
        let span = node_to_span_cached(&node, source, &line_index);

        out.push(ParsedDecl {
            name,
            params,
            is_export,
            directive_ctx,
            return_type: None,
            return_facts: None,
            span,
        });
    }

    Ok(out)
}
