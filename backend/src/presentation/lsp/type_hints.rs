//! Type Hints provider for LSP

// Placeholders and stubs for future implementation

#![allow(unused_variables)]
#![allow(dead_code)]

use bsl_shared::domain::types::TypeContext;
use tower_lsp::lsp_types::*;

pub struct TypeHintsProvider;

impl TypeHintsProvider {
    pub fn get_type_hints(
        program: &crate::parsing::bsl::ast::Program,
        type_context: &TypeContext,
    ) -> Vec<SemanticToken> {
        // Placeholder implementation
        vec![]
    }
}