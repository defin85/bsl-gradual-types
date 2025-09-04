//! System layer (flat structure)
//! System coordination and infrastructure

// === COMPLEX ARCHITECTURE (current) ===
pub mod analysis_cache;
pub mod computation;
pub mod coordination;
pub mod fs_utils;
pub mod memory_optimization;
pub mod parallel_analysis;
pub mod performance;

// === SIMPLIFIED ARCHITECTURE (new) ===
pub mod basic_observability;
pub mod parser_coordinator;
pub mod simple_cache;
pub mod system_coordinator;

// Re-export main components (complex - for backwards compatibility)
pub use analysis_cache::*;
pub use coordination::*;
pub use performance::*;

// Re-export simplified components (specific imports to avoid conflicts)
pub use basic_observability::{BasicObservability, SimpleMetrics, StructuredLogger};
pub use parser_coordinator::ParserCoordinator;
pub use simple_cache::{AnalysisCache, FileHash};
pub use system_coordinator::{CompletionItem, SymbolInfo, SystemCoordinator, TypeSystemService};
