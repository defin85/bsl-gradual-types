//! Core type system components - DEPRECATED
//!
//! This module provides temporary compatibility exports.
//! All modules have been moved to the flat architecture layers:
//! - Analysis components → domain/analysis/
//! - Resolvers → domain/resolvers/
//! - System components → system/
//! - Presentation components → presentation/
//! - Application components → application/

// Temporary re-export of types from domain layer for compatibility
pub use crate::domain::types;

// === ANALYSIS COMPONENTS ===
// Re-export from domain/analysis/
pub use crate::domain::analysis::dependency_graph;
pub use crate::domain::analysis::facets;
pub use crate::domain::analysis::flow_sensitive;
pub use crate::domain::analysis::interprocedural;
pub use crate::domain::analysis::type_checker;
pub use crate::domain::analysis::type_narrowing;
pub use crate::domain::analysis::union_types;
pub use crate::system::analysis_cache;

// === DOMAIN COMPONENTS ===
pub use crate::domain::context;
pub use crate::domain::contracts;
pub use crate::domain::resolution_service as resolution;
pub use crate::domain::standard_types;
pub use crate::domain::type_system_service;
pub use crate::domain::unified_type_system;

// === RESOLVERS ===
pub use crate::domain::resolvers::platform as platform_resolver;

// === SYSTEM COMPONENTS ===
pub use crate::system::fs_utils;
pub use crate::system::memory_optimization;
pub use crate::system::parallel_analysis;
pub use crate::system::performance;

// === PRESENTATION COMPONENTS ===
pub use crate::presentation::position;
pub use crate::presentation::type_hints;

// === APPLICATION COMPONENTS ===
pub use crate::application::code_actions;
pub use crate::application::lsp_enhanced;
