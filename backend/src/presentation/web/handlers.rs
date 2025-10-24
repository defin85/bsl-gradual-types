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

use crate::application::TypeSystemService;
use crate::system::SystemCoordinator;

// --- СТАРЫЕ DTO УДАЛЕНЫ ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<usize>,     // 1-based page number
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub certainty_level: Option<String>,
    pub flow_sensitive_only: Option<bool>,
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
        "service": "bsl-gradual-types"
    }))
}

/// Validate code fragment
/// Phase 4: TypeValidator integration - проверяет методы и свойства
pub async fn validate_code(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    use std::time::Instant;

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
