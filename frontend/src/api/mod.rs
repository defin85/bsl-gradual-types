//! API integration utilities

pub mod client;
pub mod extensions;

// Re-export shared DTOs as the source of truth
pub use bsl_shared::api::dtos::*;

// Re-export frontend-specific extensions
pub use extensions::*;

// Re-export client functions
pub use client::{fetch_metrics, fetch_snapshot_meta, fetch_type_graph, fetch_types, reload_snapshot};
