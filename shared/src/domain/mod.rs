//! The core domain logic of the type system.

pub mod analysis;
pub mod metadata_lookup; // Bridge between TypeResolution and RawTypeData
pub mod repository;
pub mod resolver;
pub mod types;
pub mod validators; // Type validation rules from Balyuk & Popova (2021)

// Re-export key components for easier access
pub use metadata_lookup::TypeMetadataLookup;
pub use repository::{CompletionItem, CompletionKind, TypeRepository};
pub use resolver::TypeResolver;
pub use types::{RawTypeData, TypeResolution};
pub use validators::{TypeValidator, TypeErrorKind};