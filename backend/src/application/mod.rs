//! Application layer (Phase 4: API Unification)
//! Unified TypeSystemService as single entry point for all clients

pub mod ast_to_ir;
pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP
pub mod type_inference_service;
pub mod type_system_service; // Milestone 2.8: AST → IR конвертер

// === Phase 4: Unified API ===
// Main unified service - single entry point for all clients
pub use bsl_shared::domain::CompletionItem;
pub use type_inference_service::TypeInferenceService;
pub use type_system_service::TypeSystemService;
