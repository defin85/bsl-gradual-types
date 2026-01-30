//! Formatters - functions for formatting type information for display
//!
//! Provides formatting utilities for hover tooltips, type descriptions,
//! and other user-facing type information.

pub mod hover_formatters;
pub mod type_formatters;

// Note: Re-exports are intentionally kept even if not all are used directly
// from this module, as they provide a convenient public API for external consumers.
#[allow(unused_imports)]
pub use hover_formatters::{
    format_concrete_type_name, format_generic_hover, format_semantic_node_info,
    format_type_for_hover, format_variable_hover,
};
#[allow(unused_imports)]
pub use type_formatters::format_resolution_result;
