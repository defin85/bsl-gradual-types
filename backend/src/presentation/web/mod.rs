//! Web Interface presentation layer
//!
//! Интегрированная архитектура: Backend = REST API + Static WASM files.
//! Обслуживает и API endpoints (/api/*) и статические файлы frontend.

pub mod handlers;
pub mod router;

pub use handlers::{AppState, SearchQuery};
pub use router::create_router;
