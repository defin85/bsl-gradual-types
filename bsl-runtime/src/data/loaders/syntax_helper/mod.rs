//! Модули парсера синтакс-помощника 1С
//!
//! Единая точка входа для работы с синтакс-помощником.
//! Содержит загрузчик (SyntaxHelperLoader) и вспомогательные модули.

pub mod document_parsers;
pub mod handlers;
pub mod html_extractors;
pub mod indexing;
pub mod loader;
pub mod stats;
pub mod type_parser;
pub mod types;
pub mod utils;

// Публичные реэкспорты для удобства использования
pub use document_parsers::DocumentParser;
pub use html_extractors::HtmlExtractor;
pub use indexing::IndexBuilder;
pub use loader::SyntaxHelperLoader;
pub use stats::ParsingStats;
pub use type_parser::{TypeFragment, TypeParser, UNION_SEPARATOR};
pub use types::*;
pub use utils::*;
