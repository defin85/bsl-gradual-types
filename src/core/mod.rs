//! Core type system components

pub mod types;
pub mod resolution;
pub mod contracts;
pub mod facets;
pub mod context;
pub mod fs_utils;
pub mod position;
pub mod platform_resolver;
pub mod dependency_graph;
pub mod type_checker;
pub mod standard_types;
pub mod type_narrowing;
pub mod flow_sensitive;
pub mod union_types;
pub mod interprocedural;
pub mod lsp_enhanced;
pub mod performance;
pub mod analysis_cache;
pub mod parallel_analysis;
pub mod memory_optimization;
pub mod code_actions;
pub mod type_hints;
pub mod type_system_service;
pub mod unified_type_system;