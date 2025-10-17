//! Web router configuration
//!
//! Настройка маршрутов для веб-сервера

use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
};
use tower_http::services::{ServeDir, ServeFile};
use std::collections::HashMap;

use super::handlers::{AppState, get_metrics, get_types, search_types, health_check, validate_code};
use crate::presentation::semantic_routes::semantic_routes;

/// Handler for favicon.ico - returns 204 No Content to avoid 404 errors
async fn favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// Create web application router
pub fn create_router(app_state: AppState, static_path: &str, enable_cors: bool) -> Router {
    let index_path = format!("{}/index.html", static_path);

    // Configure MIME types for WASM and JS files
    let mut mime_overrides = HashMap::new();
    mime_overrides.insert("wasm", "application/wasm");
    mime_overrides.insert("js", "application/javascript");

    let static_dir = ServeDir::new(static_path)
        .not_found_service(ServeFile::new(&index_path))
        .append_index_html_on_directories(true);

    // Создаём semantic router с type_service (MILESTONE 2.16)
    let semantic_router = semantic_routes(app_state.type_service.clone());

    let mut app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/metrics", get(get_metrics))
        .route("/api/types", get(get_types))
        .route("/api/search", get(search_types))
        .route("/api/validate", post(validate_code))  // New validation endpoint
        .route("/favicon.ico", get(favicon))
        .nest_service("/", semantic_router)  // ✅ MILESTONE 2.16: Semantic Visualization API
        .fallback_service(static_dir)
        .with_state(app_state);
    
    // Add CORS if enabled
    if enable_cors {
        use tower_http::cors::CorsLayer;
        app = app.layer(CorsLayer::permissive());
    }

    app
}