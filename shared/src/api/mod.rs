// shared/src/api/mod.rs

pub mod dtos {
    pub use bsl_api_dtos::dtos::*;
}

pub mod semantic_dtos {
    pub use bsl_api_dtos::semantic_dtos::*;
}

// Re-export all DTOs for easy access from other crates
pub use dtos::*;
pub use semantic_dtos::*;
