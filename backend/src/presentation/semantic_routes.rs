use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::TypeSystemService;
use crate::presentation::semantic_html_generator::{generate_semantic_html, RenderOptions, Theme};
use crate::system::fs_utils::read_bsl_file;

/// Query параметры для semantic API
#[derive(Debug, Deserialize)]
pub struct SemanticQuery {
    /// Формат ответа: "json" (по умолчанию) или "html"
    #[serde(default = "default_format")]
    pub format: String,

    /// Тема для HTML: "light" или "dark"
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Компактный режим для HTML
    #[serde(default)]
    pub compact: bool,
}

fn default_format() -> String {
    "json".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

/// Ответ в случае ошибки
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Создаёт router для semantic visualization API
///
/// # Endpoints
/// - `GET /api/semantic/:file_path` - получить семантическое дерево
///   - Поддерживает query параметры: `?format=html&theme=dark&compact=false`
///
/// # Examples
/// ```text
/// GET /api/semantic/test.bsl?format=json
/// GET /api/semantic/test.bsl?format=html&theme=dark
/// ```
pub fn semantic_routes(type_service: Arc<TypeSystemService>) -> Router {
    Router::new()
        .route("/api/semantic/:file_path", get(get_semantic_tree))
        .with_state(type_service)
}

/// Handler для получения семантического дерева
///
/// # Arguments
/// - `file_path` - путь к файлу (из URL)
/// - `params` - query параметры (format, theme, compact)
/// - `type_service` - сервис типизации
///
/// # Response
/// - `format=json` → возвращает JSON информацию
/// - `format=html` → возвращает HTML визуализацию
///
/// # Examples
/// ```text
/// GET /api/semantic/cli/test_semantic_viz.bsl?format=json
/// GET /api/semantic/cli/test_semantic_viz.bsl?format=html&theme=dark
/// ```
async fn get_semantic_tree(
    Path(file_path): Path<String>,
    Query(params): Query<SemanticQuery>,
    State(type_service): State<Arc<TypeSystemService>>,
) -> impl IntoResponse {
    // Step 1: Read the file
    let source = match read_bsl_file(std::path::Path::new(&file_path)) {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::NOT_FOUND, format!("Failed to read file: {}", e)).into_response()
        }
    };

    // Step 2: Parse to SemanticProgram
    let program = match type_service.parse_semantic_program(&source).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Parse error: {}", e),
            )
                .into_response()
        }
    };

    // Step 3: Generate HTML or JSON
    if params.format == "html" {
        let theme = match params.theme.as_str() {
            "light" => Theme::Light,
            _ => Theme::Dark,
        };
        let options = RenderOptions {
            theme,
            compact: params.compact,
        };
        let html = generate_semantic_html(&program, &file_path, options);
        (StatusCode::OK, Html(html)).into_response()
    } else {
        // JSON response - using public API methods for encapsulation
        let json_response = serde_json::json!({
            "file_path": file_path,
            "nodes_count": program.nodes.len(),
            "functions_count": program.symbols.functions_count(),
            "procedures_count": program.symbols.procedures_count(),
        });
        (StatusCode::OK, Json(json_response)).into_response()
    }
}
