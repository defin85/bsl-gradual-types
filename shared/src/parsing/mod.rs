//! Parsing abstraction layer (Milestone 2.8: Task 3)
//!
//! Parser trait для инверсии зависимостей:
//! - shared не зависит от backend
//! - CLI может использовать любую имплементацию Parser
//! - Легко тестировать с mock parser

use crate::ir::SemanticProgram;
use anyhow::Result;

/// Абстракция парсера для инверсии зависимостей
///
/// # Цель
/// Позволить CLI и другим компонентам работать с парсером через trait,
/// не завися от тяжелого backend с tree-sitter
///
/// # Примеры
///
/// ```no_run
/// use bsl_shared::parsing::Parser;
/// use bsl_shared::ir::SemanticProgram;
///
/// fn analyze_code(parser: &dyn Parser, code: &str) -> anyhow::Result<SemanticProgram> {
///     parser.parse_to_ir(code, "test.bsl")
/// }
/// ```
pub trait Parser: Send + Sync {
    /// Парсинг кода → Semantic IR
    ///
    /// # Arguments
    /// * `content` - исходный код BSL
    /// * `file_path` - путь к файлу для диагностики
    ///
    /// # Returns
    /// SemanticProgram с символами, узлами и CFG (если доступен)
    ///
    /// # Errors
    /// Возвращает ошибку если парсинг не удался
    fn parse_to_ir(&self, content: &str, file_path: &str) -> Result<SemanticProgram>;

    /// Парсинг с кешированием (опциональная оптимизация)
    ///
    /// По умолчанию просто вызывает parse_to_ir, но backend может переопределить
    /// для использования AnalysisCache
    fn parse_to_ir_cached(&self, content: &str, file_path: &str) -> Result<SemanticProgram> {
        self.parse_to_ir(content, file_path)
    }

    /// Получить имя реализации парсера (для диагностики)
    fn parser_name(&self) -> &'static str;

    /// Поддерживает ли парсер инкрементальный парсинг
    fn supports_incremental(&self) -> bool {
        false
    }
}

/// Mock парсер для тестирования
///
/// Возвращает пустую SemanticProgram без реального парсинга
pub struct MockParser;

impl Parser for MockParser {
    fn parse_to_ir(&self, content: &str, file_path: &str) -> Result<SemanticProgram> {
        use crate::ir::{SemanticProgram, SourceInfo, SymbolTable};

        let content_hash = crate::utils::hash::hash_content(content);

        Ok(SemanticProgram {
            symbols: SymbolTable::new(),
            nodes: vec![],
            source_info: SourceInfo {
                path: file_path.to_string(),
                content_hash,
            },
            cfg: None,
        })
    }

    fn parser_name(&self) -> &'static str {
        "MockParser"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_parser() {
        let parser = MockParser;
        let result = parser.parse_to_ir("", "test.bsl");

        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.nodes.len(), 0);
        assert_eq!(program.source_info.path, "test.bsl");
    }

    #[test]
    fn test_parser_name() {
        let parser = MockParser;
        assert_eq!(parser.parser_name(), "MockParser");
    }

    #[test]
    fn test_supports_incremental_default() {
        let parser = MockParser;
        assert!(!parser.supports_incremental());
    }
}
