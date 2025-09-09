//! BSL Gradual Type System - Frontend
//!
//! Leptos-based web frontend for type system visualization and interaction.

pub mod api;
pub mod components;
pub mod pages;
pub mod utils;

// Re-export main app component
pub use pages::app::App;
