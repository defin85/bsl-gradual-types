//! The core domain logic of the type system.

pub mod analysis;
pub mod repository;
pub mod resolver; // <-- НОВЫЙ МОДУЛЬ
pub mod types;

// Re-export key components for easier access
pub use repository::{CompletionItem, CompletionKind, TypeRepository};
pub use resolver::{TypeResolver};
pub use types::{RawTypeData, TypeResolution};