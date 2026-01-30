//! Extractors - stateless helper functions for symbol and type extraction
//!
//! These functions extract information from source code and AST without
//! requiring access to any stateful application service.

pub mod symbol_extractor;
pub mod type_extractor;

// Note: Re-exports are intentionally kept even if not all are used directly
// from this module, as they provide a convenient public API for external consumers.
#[allow(unused_imports)]
pub use symbol_extractor::{extract_word_at_position, is_identifier_char, utf16_to_byte_offset};
#[allow(unused_imports)]
pub use type_extractor::{
    expression_to_type_name, extract_function_name, extract_return_type,
    extract_type_from_var_declaration, extract_var_name,
};
