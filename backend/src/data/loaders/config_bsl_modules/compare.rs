use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::system::fs_utils::read_bsl_file;

use super::ast_fallback::parse_bsl_module_ast_with_progress;
use super::parsing::parse_with_thread_parser;
use super::single_pass::parse_bsl_module_tree_sitter_with_mode;
use super::types::{
    CallSite, CallTarget, ModuleParseComparison, ModuleParseStats, ParsedDecl, ParsedModuleData,
    SinglePassMode,
};

pub fn compare_module_parsing_from_file(path: &Path) -> Result<ModuleParseComparison> {
    compare_module_parsing_from_file_with_progress_mode(path, SinglePassMode::Full, |_| {})
}

pub fn compare_module_parsing_from_file_with_progress(
    path: &Path,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseComparison> {
    compare_module_parsing_from_file_with_progress_mode(path, SinglePassMode::Full, &mut progress)
}

pub fn compare_module_parsing_from_file_with_progress_mode(
    path: &Path,
    mode: SinglePassMode,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseComparison> {
    progress("Читаем исходник");
    let source = read_bsl_file(path)?;

    progress("Парсим tree-sitter");
    let tree = parse_with_thread_parser(&source)?;

    progress("Single-pass обход");
    let single_pass = parse_bsl_module_tree_sitter_with_mode(&tree, &source, mode)?;

    progress("AST конвертация: старт");
    let ast = parse_bsl_module_ast_with_progress(&tree, &source, |done, total| {
        let msg = format!("AST конвертация: {}/{}", done, total);
        progress(&msg);
    })?;

    progress("Сравнение результатов");
    Ok(compare_module_data(path.to_path_buf(), single_pass, ast))
}

pub fn single_pass_module_stats_from_file_with_progress_mode(
    path: &Path,
    mode: SinglePassMode,
    mut progress: impl FnMut(&str),
) -> Result<ModuleParseStats> {
    progress("Читаем исходник");
    let source = read_bsl_file(path)?;

    progress("Парсим tree-sitter");
    let tree = parse_with_thread_parser(&source)?;

    progress("Single-pass обход");
    let single_pass = parse_bsl_module_tree_sitter_with_mode(&tree, &source, mode)?;

    Ok(module_parse_stats(&single_pass))
}

fn module_parse_stats(parsed: &ParsedModuleData) -> ModuleParseStats {
    let export_decls = parsed.decls.iter().filter(|d| d.is_export).count();
    ModuleParseStats {
        decls: parsed.decls.len(),
        export_decls,
        call_sites: parsed.call_sites.len(),
    }
}

fn compare_module_data(
    module_path: PathBuf,
    single_pass: ParsedModuleData,
    ast: ParsedModuleData,
) -> ModuleParseComparison {
    let (single_decl_keys, single_export_count) = decl_keys(&single_pass.decls);
    let (ast_decl_keys, ast_export_count) = decl_keys(&ast.decls);

    let mut missing_decls = Vec::new();
    let mut extra_decls = Vec::new();

    for key in ast_decl_keys.keys() {
        if !single_decl_keys.contains_key(key) {
            missing_decls.push(key.clone());
        }
    }
    for key in single_decl_keys.keys() {
        if !ast_decl_keys.contains_key(key) {
            extra_decls.push(key.clone());
        }
    }

    missing_decls.sort();
    extra_decls.sort();

    let mut callsite_mismatches = Vec::new();
    let single_calls = callsite_keys(&single_pass.call_sites);
    let ast_calls = callsite_keys(&ast.call_sites);

    for key in ast_calls.keys() {
        let a = ast_calls.get(key).copied().unwrap_or(0);
        let b = single_calls.get(key).copied().unwrap_or(0);
        if a != b {
            callsite_mismatches.push(format!("{} (ast={}, single={})", key, a, b));
        }
    }
    for key in single_calls.keys() {
        if !ast_calls.contains_key(key) {
            let b = single_calls.get(key).copied().unwrap_or(0);
            callsite_mismatches.push(format!("{} (ast=0, single={})", key, b));
        }
    }

    callsite_mismatches.sort();

    ModuleParseComparison {
        module_path,
        single_pass: ModuleParseStats {
            decls: single_pass.decls.len(),
            export_decls: single_export_count,
            call_sites: single_pass.call_sites.len(),
        },
        ast: ModuleParseStats {
            decls: ast.decls.len(),
            export_decls: ast_export_count,
            call_sites: ast.call_sites.len(),
        },
        missing_decls,
        extra_decls,
        callsite_mismatches,
    }
}

fn decl_keys(decls: &[ParsedDecl]) -> (HashMap<String, usize>, usize) {
    let mut out = HashMap::new();
    let mut export_count = 0;
    for d in decls {
        if d.is_export {
            export_count += 1;
        }
        let key = format!(
            "{}|{}|{}",
            d.name.to_lowercase(),
            d.params.len(),
            d.is_export
        );
        *out.entry(key).or_insert(0) += 1;
    }
    (out, export_count)
}

fn callsite_keys(call_sites: &[CallSite]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for call in call_sites {
        let key = match &call.target {
            CallTarget::LocalFunction { name } => format!("local:{}", name.to_lowercase()),
            CallTarget::QualifiedMethod { receiver, name } => format!(
                "qual:{}:{}",
                receiver.join(".").to_lowercase(),
                name.to_lowercase()
            ),
        };
        *out.entry(key).or_insert(0) += 1;
    }
    out
}
