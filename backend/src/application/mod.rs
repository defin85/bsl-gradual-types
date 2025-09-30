//! Application layer (Phase 4: API Unification)
//! Unified TypeSystemService as single entry point for all clients

pub mod type_system_service;
pub mod type_inference_service;

// === Phase 4: Unified API ===
// Main unified service - single entry point for all clients
pub use type_system_service::TypeSystemService;
pub use type_inference_service::TypeInferenceService;
pub use bsl_shared::domain::CompletionItem;
