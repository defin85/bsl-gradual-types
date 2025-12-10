//! Application layer (Phase 4: API Unification)
//! Unified TypeSystemService as single entry point for all clients

pub mod ast_to_ir;
pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP
pub mod type_inference_service;

// Phase 4.1: Modular TypeSystemService (new structure)
// Main module containing the refactored modular implementation
pub mod type_system;

// === Phase 4: Unified API ===
// Main unified service - single entry point for all clients
pub use bsl_shared::domain::CompletionItem;
pub use type_inference_service::TypeInferenceService;

// Re-export from modular type_system module
pub use type_system::{TypeSystemService, CompletionContext};
