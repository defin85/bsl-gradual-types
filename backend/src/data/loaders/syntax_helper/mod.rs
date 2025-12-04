//! Модули парсера синтакс-помощника 1С
//!
//! Разбиение большого файла syntax_helper_parser.rs на логические модули

pub mod document_parsers;
pub mod html_extractors;
pub mod indexing;
pub mod type_parser;
pub mod types;
pub mod utils;

// Публичные реэкспорты для удобства использования
pub use document_parsers::DocumentParser;
pub use html_extractors::HtmlExtractor;
pub use indexing::IndexBuilder;
pub use type_parser::{TypeFragment, TypeParser, UNION_SEPARATOR};
pub use types::*;
pub use utils::*;
