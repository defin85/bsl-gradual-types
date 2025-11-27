//! Web router configuration
//!
//! Настройка маршрутов для веб-сервера

use axum::{
    http::{header, HeaderValue, StatusCode},
    routing::{get, post},
    Router,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use super::handlers::{
    get_hover, get_metrics, get_types, health_check, search_types, validate_code, AppState,
    get_diagnostics, get_debug_ast, get_enhanced_hover,
};

/// Handler for favicon.ico - returns 204 No Content to avoid 404 errors
async fn favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// Create web application router
pub fn create_router(app_state: AppState, static_path: &str, enable_cors: bool) -> Router {
    let index_path = format!("{}/index.html", static_path);

    let static_dir = ServeDir::new(static_path)
        .not_found_service(ServeFile::new(&index_path))
        .append_index_html_on_directories(true);

    let mut app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/metrics", get(get_metrics))
        .route("/api/types", get(get_types))
        .route("/api/search", get(search_types))
        .route("/api/validate", post(validate_code)) // Code validation endpoint
        .route("/api/hover", post(get_hover)) // Hover information endpoint
        .route("/api/hover/enhanced", post(get_enhanced_hover)) // Enhanced hover with details
        .route("/api/diagnostics", post(get_diagnostics)) // Diagnostics with error separation
        .route("/api/debug/ast", post(get_debug_ast)) // AST debugging endpoint
        .route("/favicon.ico", get(favicon))
        .fallback_service(static_dir)
        .with_state(app_state);

    // Add CORS and Cache-Control headers for development mode
    if enable_cors {
        use tower_http::cors::CorsLayer;
        app = app
            .layer(CorsLayer::permissive())
            // Disable browser cache for development - prevents stale index.html issues
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-cache, no-store, must-revalidate"),
            ));
    }

    app
}
