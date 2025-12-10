//! Loaders - configuration and type loading operations
//!
//! Functions for loading types from 1C configuration and other sources.

pub mod configuration_loader;

// Note: Re-export intentionally kept for public API convenience
#[allow(unused_imports)]
pub use configuration_loader::load_configuration_types;
