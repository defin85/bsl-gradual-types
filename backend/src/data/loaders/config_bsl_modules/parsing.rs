use std::cell::RefCell;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tree_sitter::Parser;

use crate::system::tree_sitter_adapter::TreeSitterAdapter;

use super::ast_fallback::collect_decls_and_call_sites;
use super::metrics::human_duration;
use super::single_pass::parse_bsl_module_tree_sitter_with_mode;
use super::types::{ParsedModuleData, SinglePassMode};

pub(crate) fn parse_bsl_module(source: &str, module_path: &Path) -> Result<ParsedModuleData> {
    let ts_parse_started = Instant::now();
    let tree = parse_with_thread_parser(source)?;
    let ts_parse_elapsed = ts_parse_started.elapsed();
    if ts_parse_elapsed >= Duration::from_secs(1) {
        tracing::debug!(
            "tree-sitter parse: {} ({} байт, {} строк) {:?}",
            human_duration(ts_parse_elapsed),
            source.len(),
            source.lines().count(),
            module_path
        );
    }

    let convert_started = Instant::now();
    let parse_result = parse_bsl_module_tree_sitter_with_mode(&tree, source, SinglePassMode::Full);
    let convert_elapsed = convert_started.elapsed();
    if convert_elapsed >= Duration::from_secs(1) {
        tracing::debug!(
            "tree-sitter single-pass: {} ({} байт, {} строк) {:?}",
            human_duration(convert_elapsed),
            source.len(),
            source.lines().count(),
            module_path
        );
    }

    match parse_result {
        Ok(data) => Ok(data),
        Err(e) => {
            tracing::debug!(
                "Tree-sitter single-pass failed, fallback to AST ({}): {}",
                module_path.display(),
                e
            );

            let parse_result = TreeSitterAdapter::convert_tree_fast(&tree, source)
                .map_err(|e| anyhow!("tree-sitter convert_tree_fast failed: {}", e))?;
            let (decls, call_sites) =
                collect_decls_and_call_sites(&parse_result.program.statements);
            Ok(ParsedModuleData { decls, call_sites })
        }
    }
}

pub(crate) fn parse_with_thread_parser(source: &str) -> Result<tree_sitter::Tree> {
    thread_local! {
        static THREAD_PARSER: RefCell<Option<Parser>> = const { RefCell::new(None) };
    }

    THREAD_PARSER.with(|cell| {
        let mut parser_opt = cell.borrow_mut();
        if parser_opt.is_none() {
            let mut parser = Parser::new();
            parser
                .set_language(&tree_sitter_bsl::LANGUAGE.into())
                .map_err(|e| anyhow!("tree-sitter-bsl language error: {:?}", e))?;
            *parser_opt = Some(parser);
        }

        let parser = match parser_opt.as_mut() {
            Some(parser) => parser,
            None => return Err(anyhow!("thread-local parser initialization failed")),
        };
        parser
            .parse(source, None)
            .ok_or_else(|| anyhow!("tree-sitter parse returned None"))
    })
}
