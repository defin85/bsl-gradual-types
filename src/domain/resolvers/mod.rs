//! Resolvers for different type sources

pub(crate) mod platform;

// CompletionItem и CompletionKind нужно экспортировать для TypeResolutionService
pub use platform::{CompletionItem, CompletionKind};
