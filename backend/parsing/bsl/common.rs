//! Общие типы и трейты для парсеров

use super::ast::Program;
use anyhow::Result;

/// Общий trait для всех парсеров BSL
pub trait Parser: Send + Sync {
    /// Парсить исходный код и вернуть AST
    fn parse(&mut self, source: &str) -> Result<Program>;

    /// Парсить инкрементально (для LSP)
    fn parse_incremental(&mut self, source: &str, _changes: &[TextChange]) -> Result<Program> {
        // По умолчанию просто перепарсиваем весь файл
        self.parse(source)
    }

    /// Получить имя парсера
    fn name(&self) -> &str;
}

/// Изменение текста для инкрементального парсинга
#[derive(Debug, Clone)]
pub struct TextChange {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Position,
    pub old_end_position: Position,
    pub new_end_position: Position,
}

/// Позиция в тексте
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

/// Фабрика для создания парсеров
pub struct ParserFactory;

impl ParserFactory {
    /// Создать парсер по умолчанию
    pub fn create() -> Box<dyn Parser> {
        Self::create_tree_sitter()
    }

    /// Создать парсер (всегда tree-sitter)
    fn create_tree_sitter() -> Box<dyn Parser> {
        // TEMPORARY: Disabled during architecture simplification
        // Box::new(crate::parsing::bsl::tree_sitter_adapter::TreeSitterAdapter::new().unwrap())
        Box::new(crate::parsing::bsl::parser::BslParser::new("").unwrap())
    }

    /// Создать парсер по имени
    pub fn create_by_name(name: &str) -> Option<Box<dyn Parser>> {
        match name {
            "tree-sitter" => {
                // TEMPORARY: Disabled during architecture simplification
                // crate::parsing::bsl::tree_sitter_adapter::TreeSitterAdapter::new()
                match crate::parsing::bsl::parser::BslParser::new("") {
                    Ok(parser) => Some(Box::new(parser) as Box<dyn Parser>),
                    Err(_) => None,
                }
            }
            _ => None,
        }
    }

    /// Получить список доступных парсеров
    pub fn available_parsers() -> Vec<&'static str> {
        vec!["tree-sitter"]
    }
}
