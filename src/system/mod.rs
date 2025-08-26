//! System layer (flat structure)
//! System coordination and infrastructure

pub mod analysis_cache;
pub mod computation;
pub mod coordination;
pub mod fs_utils;
pub mod memory_optimization;
pub mod parallel_analysis;
pub mod performance;

// Re-export main components
pub use analysis_cache::*;
pub use coordination::*;
pub use performance::*;
