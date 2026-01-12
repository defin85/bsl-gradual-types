//! System layer (flat structure)
//! System coordination and infrastructure

// === SIMPLIFIED ARCHITECTURE ONLY ===
pub mod basic_observability;
pub mod ast_cache;
pub mod disk_cache; // Milestone D1: Disk cache API
pub mod deps_bundle_v2; // IntelliSense v2 deps snapshot bundle
pub mod fs_utils; // Keep utility functions
pub mod intellisense_index; // M2: IntelliSense indexes
pub mod intellisense_index_store; // M2: IntelliSense indexes storage
pub mod keyword_index; // M2: Keyword index sources
pub mod parallel_analyzer; // Milestone 2.4: Параллельный анализ
pub mod parser_coordinator;
pub mod persistent_cache; // Milestone 2.4: Межсессионное кеширование
pub mod positioning; // UTF-16 <-> byte offsets for LSP/tree-sitter
pub mod system_coordinator;
pub mod tree_cache;
pub mod tree_sitter_adapter;

// Re-export simplified components (specific imports to avoid conflicts)
pub use basic_observability::{BasicObservability, SimpleMetrics, StructuredLogger};
pub use ast_cache::{AstCache, AstCacheStats};
pub use disk_cache::{
    CacheCleanupReport, CacheEntry, CacheManifest, DiskCache, DiskCacheKey,
    DiskCacheStatsSnapshot,
};
pub use deps_bundle_v2::{DepsBundleV2, DepsBundleV2Meta, build_deps_bundle_v2};
pub use intellisense_index::{
    IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IndexSnapshotId, IntellisenseIndexStore,
    SymbolKind, SymbolScope, TypeKind, Visibility, INDEX_SCHEMA_VERSION,
};
pub use intellisense_index_store::{
    IndexStoreVersion, IntellisenseIndexDiskStore, INDEX_STORE_VERSION,
};
pub use parallel_analyzer::{ParallelAnalyzer, PerformanceStats, ProjectAnalysisResult};
pub use parser_coordinator::ParserCoordinator;
pub use positioning::{LineIndex, byte_offset_to_utf16, utf16_to_byte_offset};
pub use persistent_cache::{CacheCleanupStats, CacheStats, CachedAnalysis, PersistentCache};
pub use system_coordinator::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, ConfigIndexCache,
    DiskCacheStatsReport, LoadMetadataResult, ObjectKey, StartupError, SymbolInfo,
    SystemCoordinator,
};
pub use tree_cache::{hash_content, TreeCache};
pub use tree_sitter_adapter::TreeSitterAdapter;
