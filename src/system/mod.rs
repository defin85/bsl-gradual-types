//! System layer (flat structure)
//! System coordination and infrastructure

// === SIMPLIFIED ARCHITECTURE ONLY ===
pub mod basic_observability;
pub mod fs_utils; // Keep utility functions
pub mod parser_coordinator;
pub mod simple_cache;
pub mod system_coordinator;

// Re-export simplified components (specific imports to avoid conflicts)
pub use basic_observability::{BasicObservability, SimpleMetrics, StructuredLogger};
pub use parser_coordinator::ParserCoordinator;
pub use simple_cache::{AnalysisCache, AnalysisResult, FileHash};
pub use system_coordinator::{SymbolInfo, SystemCoordinator};
