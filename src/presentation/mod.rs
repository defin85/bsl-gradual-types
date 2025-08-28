//! Presentation layer (flat structure)  
//! Protocol adapters and UI components

pub mod adapters;
pub mod position;
pub mod type_hints;
pub mod web_ui;

// Re-export main components
pub use adapters::*;
