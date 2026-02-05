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
    use bsl_shared::domain::types::ErrorType;
    use crate::tree_sitter_adapter::span::LineIndex;

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
    fn parse_reports_byte_spans_for_incomplete_new_with_emoji_prefix() {
        let source = "Процедура Тест()\n😀 = Новый\nКонецПроцедуры";
        let result = parse(source, &ParseOptions::default()).unwrap();

        let err = result
            .syntax_errors
            .iter()
            .find(|e| e.message.contains("Отсутствует тип после 'Новый'"))
            .expect("expected incomplete 'Новый' diagnostic");

        let line = "😀 = Новый";
        let start_of_line = source.find(line).expect("line offset");
        let expected_offset = (start_of_line + line.len()) as u32;

        assert_eq!(err.span.start, expected_offset);
        assert_eq!(err.span.end, expected_offset);
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

    #[test]
    fn parse_rewrites_for_step_clause_span_to_step_keyword() {
        let source = "Для Индекс = 10 По 0 Шаг -1 Цикл\nКонецЦикла";
        let result = parse(source, &ParseOptions::default()).unwrap();

        let err = result
            .syntax_errors
            .iter()
            .find(|e| e.message.contains("Шаг <expr>") || e.message.contains("Шаг"))
            .expect("expected rewritten diagnostic for `Шаг` clause");

        assert_eq!(err.error_type, ErrorType::InvalidSyntax);
        let step_offset = source.find("Шаг").expect("Шаг offset") as u32;
        assert_eq!(err.span.start, step_offset);
        assert_eq!(err.span.end, step_offset + "Шаг".len() as u32);
    }

    #[test]
    fn parse_valid_for_header_has_no_syntax_errors() {
        let source = "Для Индекс = 10 По 0 Цикл\nКонецЦикла";
        let result = parse(source, &ParseOptions::default()).unwrap();
        assert!(
            result.syntax_errors.is_empty(),
            "expected no syntax errors, got: {:?}",
            result.syntax_errors
        );
    }

    #[test]
    fn parse_if_without_then_reports_single_helpful_error_on_header_line() {
        let source = "Если x = 1\n    Сообщить(x);\nКонецЕсли;";
        let result = parse(source, &ParseOptions::default()).unwrap();
        assert!(!result.syntax_errors.is_empty());

        let line_index = LineIndex::new(source);

        let err = result
            .syntax_errors
            .iter()
            .find(|e| e.message.contains("Тогда"))
            .expect("expected rewritten diagnostic mentioning `Тогда`");

        assert_eq!(err.error_type, ErrorType::InvalidSyntax);

        let (line, _) = line_index.byte_offset_to_point(source, err.span.start as usize);
        assert_eq!(line, 0, "expected error to point to `Если` header line");

        let header = "Если x = 1";
        let expected = header.len() as u32;
        assert_eq!(err.span.start, expected);
        assert_eq!(err.span.end, expected);

        let errors_on_header_line = result
            .syntax_errors
            .iter()
            .filter(|e| {
                line_index
                    .byte_offset_to_point(source, e.span.start as usize)
                    .0
                    == 0
            })
            .count();
        assert_eq!(errors_on_header_line, 1);
    }

    #[test]
    fn parse_unclosed_try_is_rewritten_and_preserves_related_info() {
        let source =
            "Попытка\n    Сообщить(1);\nИсключение\n    Сообщить(2);\nСообщить(3);\n";
        let result = parse(source, &ParseOptions::default()).unwrap();
        assert!(!result.syntax_errors.is_empty());

        let line_index = LineIndex::new(source);

        let err = result
            .syntax_errors
            .iter()
            .find(|e| e.message.contains("КонецПопытки"))
            .unwrap_or_else(|| {
                panic!(
                    "expected rewritten diagnostic for unclosed `Попытка`, got: {:?}",
                    result.syntax_errors
                )
            });

        assert_eq!(err.error_type, ErrorType::MissingToken);
        if !err.related.is_empty() {
            assert!(
                err.related
                    .iter()
                    .any(|r| r.message.contains("Начало блока: Попытка")),
                "expected related info to include try start, got: {:?}",
                err.related
            );
        }

        let (line, _) = line_index.byte_offset_to_point(source, err.span.start as usize);
        assert_eq!(line, 0);
        assert_eq!(err.span.start, 0);

        let errors_on_try_line = result
            .syntax_errors
            .iter()
            .filter(|e| {
                line_index
                    .byte_offset_to_point(source, e.span.start as usize)
                    .0
                    == 0
            })
            .count();
        assert_eq!(errors_on_try_line, 1);
    }
}
