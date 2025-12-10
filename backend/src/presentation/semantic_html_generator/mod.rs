//! Semantic HTML Generator - Pure HTML visualization for BSL semantic trees
//!
//! Generates self-contained HTML with inline CSS for displaying semantic program structures.
//! Uses inline CSS only (no CDN) due to VSCode Content Security Policy restrictions.
//!
//! # Module Structure
//! - `generator` - Main HTML generation logic and public types
//! - `renderers` - Rendering functions for nodes and symbol tables
//! - `styles` - CSS styles and color schemes
//! - `utils` - Utility functions (HTML escaping)

mod generator;
mod renderers;
mod styles;
mod utils;

// Re-export public API
pub use generator::{generate_semantic_html, RenderOptions, Theme};
