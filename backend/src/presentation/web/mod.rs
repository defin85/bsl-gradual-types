//! Web API presentation layer
//!
//! Backend только предоставляет REST API для frontend.
//! UI компоненты находятся в frontend/ крейте.

pub mod handlers;
pub mod router;

pub use handlers::{AppState, ApiMetrics, ApiType, ApiUnionType, SearchQuery};
pub use router::create_router;
