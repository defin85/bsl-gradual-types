// shared/src/api/mod.rs

pub mod dtos;
pub mod semantic_dtos;

// Re-export all DTOs for easy access from other crates
pub use dtos::*;
pub use semantic_dtos::*;
