//! System layer (flat structure)
//! System coordination and infrastructure

// === SIMPLIFIED ARCHITECTURE ONLY ===
pub mod basic_observability;
pub mod fs_utils; // Keep utility functions
pub mod ir_cache; // Milestone 2.13: IR кеширование для LSP hover
pub mod parallel_analyzer; // Milestone 2.4: Параллельный анализ
pub mod parser_coordinator;
pub mod persistent_cache; // Milestone 2.4: Межсессионное кеширование
pub mod simple_cache;
pub mod system_coordinator;
pub mod tree_cache;
pub mod tree_sitter_adapter;

// Re-export simplified components (specific imports to avoid conflicts)
pub use basic_observability::{BasicObservability, SimpleMetrics, StructuredLogger};
pub use ir_cache::{IrCache, IrCacheStats};
pub use parallel_analyzer::{ParallelAnalyzer, ProjectAnalysisResult, PerformanceStats};
pub use parser_coordinator::ParserCoordinator;
pub use persistent_cache::{CachedAnalysis, PersistentCache, CacheStats, CacheCleanupStats};
pub use simple_cache::{AnalysisCache, AnalysisResult, FileHash};
pub use system_coordinator::{SymbolInfo, SystemCoordinator};
pub use tree_cache::{hash_content, TreeCache};
pub use tree_sitter_adapter::TreeSitterAdapter;
