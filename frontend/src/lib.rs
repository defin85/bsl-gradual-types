//! BSL Gradual Type System - Frontend
//!
//! Leptos-based web frontend for type system visualization and interaction.

pub mod api;
pub mod components;
pub mod config;
pub mod pages;
pub mod utils;
pub mod app;

// Re-export main app component
pub use app::App;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Main entry point for WASM application
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    
    // Initialize configuration
    config::init_config();
    
    mount_to_body(|| view! { <App/> })
}
