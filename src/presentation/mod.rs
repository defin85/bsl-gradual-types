//! Presentation layer (flat structure)  
//! Protocol adapters and UI components

pub mod adapters;
pub mod interfaces;
pub mod position;
pub mod type_hints;

// Re-export main components
pub use adapters::*;
pub use interfaces::*;