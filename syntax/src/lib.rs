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
mod tests {
    use super::*;

    #[test]
    fn parse_valid_code_has_no_syntax_errors() {
        let source = "Процедура Тест()\nКонецПроцедуры";
        let result = parse(source, &ParseOptions::default()).unwrap();
        assert!(result.syntax_errors.is_empty());
        assert!(!result.program.statements.is_empty());
    }

    #[test]
    fn parse_broken_code_returns_partial_result_with_errors() {
        let source = "Процедура Тест(\nКонецПроцедуры";
        let result = parse(source, &ParseOptions::default()).unwrap();
        assert!(!result.syntax_errors.is_empty());
    }

    #[test]
    fn parse_reports_utf16_spans_for_incomplete_new_with_emoji_prefix() {
        let source = "Процедура Тест()\n😀 = Новый\nКонецПроцедуры";
        let result = parse(source, &ParseOptions::default()).unwrap();

        let err = result
            .syntax_errors
            .iter()
            .find(|e| e.message.contains("Отсутствует тип после 'Новый'"))
            .expect("expected incomplete 'Новый' diagnostic");

        let trimmed = "😀 = Новый";
        let expected_col_utf16: u32 = trimmed.chars().map(|c| c.len_utf16() as u32).sum();

        assert_eq!(err.span.start_line, 1);
        assert_eq!(err.span.end_line, 1);
        assert_eq!(err.span.start_column, expected_col_utf16);
        assert_eq!(err.span.end_column, expected_col_utf16);
    }

    #[test]
    fn parse_is_deterministic_for_same_input() {
        let source = "Процедура Тест(\nКонецПроцедуры";
        let a = parse(source, &ParseOptions::default()).unwrap();
        let b = parse(source, &ParseOptions::default()).unwrap();

        let project =
            |e: &bsl_shared::domain::types::ParseError| (e.error_type, e.message.clone(), e.span);

        let a_projected: Vec<_> = a.syntax_errors.iter().map(project).collect();
        let b_projected: Vec<_> = b.syntax_errors.iter().map(project).collect();

        assert_eq!(a_projected, b_projected);
    }
}
