use tree_sitter::Parser;

pub mod ast;
pub mod formatter;
pub mod tree_sitter_adapter;

pub use tree_sitter_adapter::{
    collect_syntax_errors, collect_syntax_errors_cached, TreeSitterAdapter,
};

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {}

#[derive(Debug, thiserror::Error)]
pub enum ParseFatalError {
    #[error("tree-sitter-bsl language error: {0}")]
    Language(String),
    #[error("tree-sitter parse returned None")]
    ParseFailed,
    #[error("tree-sitter adapter error: {0}")]
    Adapter(String),
}

pub fn parse(source: &str, _options: &ParseOptions) -> Result<ast::ParseResult, ParseFatalError> {
    let tree = parse_tree(source)?;
    TreeSitterAdapter::convert_tree(&tree, source).map_err(ParseFatalError::Adapter)
}

pub fn parse_fast(source: &str) -> Result<ast::ParseResult, ParseFatalError> {
    let tree = parse_tree(source)?;
    TreeSitterAdapter::convert_tree_fast(&tree, source).map_err(ParseFatalError::Adapter)
}

pub(crate) fn parse_tree(source: &str) -> Result<tree_sitter::Tree, ParseFatalError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .map_err(|e| ParseFatalError::Language(format!("{:?}", e)))?;
    parser
        .parse(source, None)
        .ok_or(ParseFatalError::ParseFailed)
}

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
