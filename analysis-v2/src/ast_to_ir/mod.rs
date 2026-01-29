//! AST -> IR conversion used by the IntelliSense v2 analysis pipeline.

mod converter;
mod expression_converter;
mod global_collections;
mod statement_converter;

#[cfg(test)]
mod tests;

pub use converter::AstToIrConverter;
pub use global_collections::{
    get_manager_type_for_metadata, is_global_collection, lookup_global_collection,
    GlobalCollectionInfo, GLOBAL_COLLECTIONS_INFO,
};
