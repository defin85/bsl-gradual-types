//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

use crate::application::TypeSystemService;
use crate::system::SystemCoordinator;
use bsl_shared::api::{
    AstNodeDto, DebugAstResponseDto, DiagnosticsResponseDto, EnhancedHoverResponse,
    SemanticErrorDto, StartupProgressDto, SyntaxErrorDto,
};

// --- СТАРЫЕ DTO УДАЛЕНЫ ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<usize>, // 1-based page number
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub certainty_level: Option<String>,
    pub flow_sensitive_only: Option<bool>,
}

#[derive(Deserialize)]
pub struct HoverRequest {
    pub code: String,
    pub line: u32,
    pub column: u32,
    #[serde(default = "default_detail_level")]
    pub detail_level: String,
}

fn default_detail_level() -> String {
    "detailed".to_string()
}

#[derive(Clone)]
pub struct AppState {
    pub type_service: Arc<TypeSystemService>,
    pub system_coordinator: Arc<SystemCoordinator>, // ВРЕМЕННО для отладки
}

/// Get system metrics
/// Phase 5: Thin handler - делегирует всю логику в TypeSystemService
pub async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.type_service.get_metrics_summary();
    Json(metrics)
}

/// Get all types with pagination support
/// Phase 5: Thin handler - делегирует всю логику в TypeSystemService
pub async fn get_types(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> impl IntoResponse {
    // Валидация и значения по умолчанию
    let page = params.page.unwrap_or(1).max(1); // Минимум страница 1
    let limit = params.limit.unwrap_or(50).clamp(1, 1000); // 1-1000 элементов

    // Конвертация page → offset для внутреннего использования
    let offset = (page - 1) * limit;

    // Вся бизнес-логика и DTO конверсия теперь в Application Layer
    let result = state.type_service.get_all_types_as_dto(
        limit,
        offset,
        params.category,
        params.certainty_level,
        params.flow_sensitive_only.unwrap_or(false),
    );
    Json(result)
}

/// Search types by query
/// Phase 5: Thin handler - делегирует всю логику в TypeSystemService
pub async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types_as_dto(&query.q).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bsl-gradual-types",
        "version": env!("CARGO_PKG_VERSION"),
        "build": env!("BUILD_TIMESTAMP"),
        "git": env!("GIT_HASH")
    }))
}

/// Version information endpoint
pub async fn get_version() -> impl IntoResponse {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "build_timestamp": env!("BUILD_TIMESTAMP"),
        "git_hash": env!("GIT_HASH"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION", "unknown"),
        "name": "BSL Gradual Types"
    }))
}

/// Startup progress endpoint (polling).
///
/// Возвращает прогресс инициализации системы, включая загрузку конфигурации и индексацию модулей.
pub async fn get_startup_progress(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot: StartupProgressDto = state.system_coordinator.startup_progress();
    Json(snapshot)
}

/// Validate code fragment
/// Phase 4: TypeValidator integration - проверяет методы и свойства
pub async fn validate_code(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    match state
        .type_service
        .validate_code_fragment(&payload.code)
        .await
    {
        Ok(errors) => {
            let is_valid = errors.is_empty();
            let duration_ms = start.elapsed().as_millis() as u64;

            let response = bsl_shared::api::ValidateCodeResponse {
                is_valid,
                errors,
                metadata: Some(bsl_shared::api::ValidationMetadataDto {
                    expressions_analyzed: 1,
                    types_resolved: if is_valid { 1 } else { 0 },
                    duration_ms,
                }),
            };

            Json(response).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get hover information for code position
/// Phase 5: LSP Hover integration - returns type information at position
pub async fn get_hover(
    State(state): State<AppState>,
    Json(req): Json<HoverRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    match state
        .type_service
        .get_hover_info(&req.code, req.line, req.column, None)
        .await
    {
        Ok(hover_text) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            let response = serde_json::json!({
                "hover": hover_text,
                "line": req.line,
                "column": req.column,
                "duration_ms": duration_ms
            });

            Json(response).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Enhanced diagnostics endpoint - separates syntax and semantic errors
/// Milestone 2.18: Comprehensive diagnostics
/// UPDATED Phase 5: Now uses validate_semantics for full semantic validation
pub async fn get_diagnostics(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    // 1) Сначала синтаксис: если есть синтаксические ошибки — семантика бессмысленна
    let syntax = match state.type_service.parse_and_validate(&payload.code) {
        Ok(errors) => errors,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let syntax_errors: Vec<SyntaxErrorDto> = syntax
        .into_iter()
        .map(|e| SyntaxErrorDto {
            message: e.message,
            line: e.span.start_line,
            column: e.span.start_column,
        })
        .collect();

    if !syntax_errors.is_empty() {
        let duration_ms = start.elapsed().as_millis();
        let response = DiagnosticsResponseDto {
            total_errors: syntax_errors.len(),
            syntax_errors,
            semantic_errors: vec![],
            duration_ms,
        };
        return Json(response).into_response();
    }

    // 2) Семантика (Phase 5: Unknown type, method/property existence)
    match state
        .type_service
        .validate_semantics(&payload.code, None)
        .await
    {
        Ok(diagnostics) => {
            let semantic_errors: Vec<SemanticErrorDto> = diagnostics
                .iter()
                .map(|d| SemanticErrorDto {
                    message: d.message.clone(),
                    line: d.line,
                    column: d.column,
                    end_line: d.end_line,
                    end_column: d.end_column,
                    severity: format!("{:?}", d.severity).to_lowercase(),
                })
                .collect();

            let duration_ms = start.elapsed().as_millis();
            let response = DiagnosticsResponseDto {
                syntax_errors: vec![],
                total_errors: semantic_errors.len(),
                semantic_errors,
                duration_ms,
            };

            Json(response).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Debug diagnostics endpoint - returns extended debug info
pub async fn get_diagnostics_debug(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let syntax = match state.type_service.parse_and_validate(&payload.code) {
        Ok(errors) => errors,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let syntax_errors: Vec<SyntaxErrorDto> = syntax
        .into_iter()
        .map(|e| SyntaxErrorDto {
            message: e.message,
            line: e.span.start_line,
            column: e.span.start_column,
        })
        .collect();

    if !syntax_errors.is_empty() {
        let duration_ms = start.elapsed().as_millis();
        return Json(serde_json::json!({
            "syntaxErrors": syntax_errors,
            "semanticErrors": [],
            "totalErrors": syntax_errors.len(),
            "durationMs": duration_ms,
            "debug": { "note": "semantic validation skipped due to syntax errors" }
        }))
        .into_response();
    }

    match state
        .type_service
        .validate_semantics_debug(&payload.code)
        .await
    {
        Ok((diagnostics, debug_info)) => {
            let semantic_errors: Vec<SemanticErrorDto> = diagnostics
                .iter()
                .map(|d| SemanticErrorDto {
                    message: d.message.clone(),
                    line: d.line,
                    column: d.column,
                    end_line: d.end_line,
                    end_column: d.end_column,
                    severity: format!("{:?}", d.severity).to_lowercase(),
                })
                .collect();

            let duration_ms = start.elapsed().as_millis();

            Json(serde_json::json!({
                "syntaxErrors": [],
                "semanticErrors": semantic_errors,
                "totalErrors": semantic_errors.len(),
                "durationMs": duration_ms,
                "debug": debug_info
            }))
            .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Debug AST endpoint - shows parsed structure for debugging
/// Milestone 2.16: Semantic visualization
pub async fn get_debug_ast(
    State(_state): State<AppState>,
    Json(_payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let duration_ms = start.elapsed().as_millis();

    // Stub implementation - returns minimal AST for testing
    let response = DebugAstResponseDto {
        nodes: vec![AstNodeDto {
            kind: "Program".to_string(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
            text: None,
        }],
        symbol_table: vec![],
        parse_errors: 0,
        duration_ms,
    };

    Json(response).into_response()
}

/// Enhanced hover endpoint - provides detailed variable information
/// Milestone 2.13: Enhanced hover with variable details
pub async fn get_enhanced_hover(
    State(state): State<AppState>,
    Json(req): Json<HoverRequest>,
) -> impl IntoResponse {
    use crate::helpers::hover_formatter::HoverFormatConfig;
    use bsl_shared::formatting::DetailLevel;

    let start = Instant::now();

    // Парсить detail_level из request
    let detail_level = DetailLevel::parse(&req.detail_level);

    // Создать конфиг с нужным detail_level
    let hover_config = HoverFormatConfig {
        detail_level,
        ..Default::default()
    };

    match state
        .type_service
        .get_hover_info(&req.code, req.line, req.column, Some(hover_config))
        .await
    {
        Ok(hover_text) => {
            let duration_ms = start.elapsed().as_millis();

            // Handle Option<String> from get_hover_info
            let hover_text_str =
                hover_text.unwrap_or_else(|| "No information available".to_string());

            let response = EnhancedHoverResponse {
                hover_text: hover_text_str,
                variable_name: None,
                variable_type: None,
                type_hint: None,
                found_in_scope: false,
                line: req.line,
                column: req.column,
                duration_ms,
            };

            Json(response).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Request для получения семантического дерева
#[derive(Deserialize)]
pub struct SemanticTreeRequest {
    pub code: String,
    #[serde(default = "default_file_path")]
    pub file_path: String,
    /// Compact режим: исключает symbol_table и call_graph для уменьшения размера ответа
    #[serde(default)]
    pub compact: bool,
    /// Включить граф вызовов (по умолчанию: true)
    #[serde(default = "default_true")]
    pub include_call_graph: bool,
    /// Включить flow-sensitive информацию (по умолчанию: true)
    #[serde(default = "default_true")]
    pub include_flow_sensitive: bool,
}

fn default_file_path() -> String {
    "inline.bsl".to_string()
}

fn default_true() -> bool {
    true
}

/// Get semantic tree for code - показывает семантическое представление кода
/// Milestone 5.3: Web API endpoint для семантического дерева
///
/// Возвращает:
/// - root_nodes: дерево семантических узлов (функции, переменные, вызовы)
/// - symbol_table: таблица символов с типами
/// - metrics: метрики анализа (количество узлов, известных/выведенных типов)
pub async fn get_semantic_tree(
    State(state): State<AppState>,
    Json(req): Json<SemanticTreeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    match state
        .type_service
        .get_semantic_tree(
            &req.code,
            &req.file_path,
            req.compact,
            req.include_call_graph,
            req.include_flow_sensitive,
        )
        .await
    {
        Ok(tree) => {
            let duration_ms = start.elapsed().as_millis();

            // Добавляем время выполнения в метрики
            let response = serde_json::json!({
                "file_path": tree.file_path,
                "root_nodes": tree.root_nodes,
                "symbol_table": tree.symbol_table,
                "metrics": {
                    "node_count": tree.metrics.node_count,
                    "procedure_count": tree.metrics.procedure_count,
                    "function_count": tree.metrics.function_count,
                    "variable_count": tree.metrics.variable_count,
                    "known_types": tree.metrics.known_types,
                    "inferred_types": tree.metrics.inferred_types,
                    "unknown_types": tree.metrics.unknown_types,
                    "analysis_time_ms": tree.metrics.analysis_time_ms,
                    "request_duration_ms": duration_ms,
                }
            });

            Json(response).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
