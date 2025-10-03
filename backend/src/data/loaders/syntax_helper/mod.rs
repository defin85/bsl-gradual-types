//! Модули парсера синтакс-помощника 1С
//!
//! Разбиение большого файла syntax_helper_parser.rs на логические модули

pub mod types;
pub mod html_extractors;
pub mod indexing;
pub mod utils;
pub mod document_parsers;

// Публичные реэкспорты для удобства использования
pub use types::*;
pub use html_extractors::HtmlExtractor;
pub use indexing::IndexBuilder;
pub use utils::*;
pub use document_parsers::DocumentParser;
