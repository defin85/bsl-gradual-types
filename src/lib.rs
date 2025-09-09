//! BSL Gradual Types - Root crate
//!
//! Re-exports for compatibility with tests and benchmarks

// Re-export specific parsers for external use
pub use bsl_backend::data::loaders::{
    ConfigurationGuidedParser, OptimizationSettings, SyntaxHelperParser,
};

// Re-export backend modules for compatibility
pub use bsl_backend::data;
pub use bsl_backend::parsing;
pub use bsl_backend::system;
