//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::application::TypeSystemService;
use crate::system::SystemCoordinator;
use bsl_shared::domain::types::Certainty;

// --- СТАРЫЕ DTO УДАЛЕНЫ ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Clone)]
pub struct AppState {
    pub type_service: Arc<TypeSystemService>,
    pub system_coordinator: Arc<SystemCoordinator>, // ВРЕМЕННО для отладки
}

/// Get system metrics (оставляем пока без изменений, но в будущем его можно будет объединить)
pub async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let all_types = state.type_service.get_all_platform_globals();

    let mut known = 0;
    let mut inferred = 0;
    let mut unknown = 0;

    for res in all_types.values() {
        match res.certainty {
            Certainty::Known => known += 1,
            Certainty::Inferred(_) => inferred += 1,
            Certainty::Unknown => unknown += 1,
        }
    }
    
    // Временная структура для совместимости
    #[derive(serde::Serialize)]
    pub struct OldApiMetrics {
        pub total_types: usize,
        pub known_types: usize,
        pub inferred_types: usize,
        pub unknown_types: usize,
    }

    Json(OldApiMetrics {
        total_types: all_types.len(),
        known_types: known,
        inferred_types: inferred,
        unknown_types: unknown,
    })
}

/// Get all types with pagination support
/// Phase 5: Thin handler - делегирует всю логику в TypeSystemService
pub async fn get_types(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0);

    // Вся бизнес-логика и DTO конверсия теперь в Application Layer
    let result = state.type_service.get_all_types_as_dto(limit, offset);
    Json(result)
}

/// Search types by query (оставляем пока без изменений)
pub async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types(&query.q).await {
        Ok(results) => Json(results).into_response(),
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