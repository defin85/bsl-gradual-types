//! Semantic layer: AST (bsl-syntax) -> SemanticProgram (bsl-shared).

mod converter;
mod expression_converter;
mod global_collections;
mod metadata_helpers;
mod statement_converter;
mod type_inference;

#[cfg(test)]
mod tests;

pub use converter::AstToIrConverter;
pub use global_collections::{
    get_manager_type_for_metadata, is_global_collection, lookup_global_collection,
    GlobalCollectionInfo, GLOBAL_COLLECTIONS_INFO,
};
