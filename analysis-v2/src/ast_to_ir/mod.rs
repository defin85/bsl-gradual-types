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
    lookup_global_collection_by_manager_type, lookup_legacy_metadata_object_collection_fallback,
    GlobalCollectionInfo, LegacyMetadataObjectCollectionFallbackInfo, GLOBAL_COLLECTIONS_INFO,
    LEGACY_METADATA_OBJECT_COLLECTION_FALLBACKS,
};
