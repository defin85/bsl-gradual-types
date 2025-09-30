//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::application::TypeSystemService;
use crate::system::SystemCoordinator;
use bsl_shared::api::dtos::{AnalysisResultDto, TypeDto, CategoryDto, MetricsDto, PaginationDto, UnionComponentDto};
use bsl_shared::domain::types::{Certainty, ResolutionResult};

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
pub async fn get_types(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> impl IntoResponse {
    // КЛЮЧЕВОЕ ИСПРАВЛЕНИЕ: используем TypeSystemService через правильный API
    let all_types = state.type_service.get_all_platform_globals();

    // ОТЛАДОЧНЫЙ ВЫВОД
    eprintln!("🔍 DEBUG: TypeSystemService вернул {} типов", all_types.len());
    for (name, _) in all_types.iter().take(5) {
        eprintln!("🔍 DEBUG: Тип: {}", name);
    }

    // Применяем пагинацию
    let limit = params.limit.unwrap_or(50).min(1000); // Максимум 1000 типов за раз
    let offset = params.offset.unwrap_or(0);

    eprintln!("🔍 DEBUG: Пагинация - limit: {}, offset: {}", limit, offset);

    // Шаг 2: Преобразуем TypeResolution в наши новые DTO с пагинацией
    let type_dtos: Vec<TypeDto> = all_types
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(name, res)| {
            // TODO: Эту логику нужно будет уточнить, пока это макет
            let (category, source) = match &res.source {
                bsl_shared::domain::types::ResolutionSource::Static => ("Platform".to_string(), "Static Analysis".to_string()),
                _ => ("Configuration".to_string(), "Configuration".to_string()),
            };

            let certainty_val = match res.certainty {
                Certainty::Known => 100,
                Certainty::Inferred(val) => (val * 100.0) as u8,
                Certainty::Unknown => 30,
            };

            let union_types = if let ResolutionResult::Union(types) = &res.result {
                Some(
                    types
                        .iter()
                        .map(|wt| UnionComponentDto {
                            type_name: format!("{:?}", wt.type_), // TODO: нужно более красивое имя
                            probability: (wt.weight * 100.0) as u8,
                        })
                        .collect(),
                )
            } else {
                None
            };

            TypeDto {
                id: name.clone(),
                name: name.clone(),
                category,
                certainty: certainty_val,
                certainty_text: format!("{:?} {}%", res.certainty, certainty_val),
                facets: res.available_facets.iter().map(|f| format!("{:?}", f)).collect(),
                methods_count: None, // TODO: Получить реальное количество
                methods: Vec::new(), // TODO: Получить несколько примеров
                attributes_count: None, // TODO: Получить реальное количество
                source,
                flow_sensitive: false, // TODO: Определить из анализа
                description: "Здесь будет подробное описание типа...".to_string(), // TODO
                union_types,
                flow_analysis: None, // TODO
                connections: None, // TODO
                warning: None, // TODO
                recommendation: None, // TODO
            }
        })
        .collect();

    // Шаг 3: Собираем остальные части AnalysisResultDto (пока это мокап)
    let metrics = MetricsDto {
        total_types: all_types.len(),
        certainty_high: type_dtos.iter().filter(|t| t.certainty > 80).count(),
        certainty_medium: type_dtos.iter().filter(|t| t.certainty > 40 && t.certainty <= 80).count(),
        certainty_low: type_dtos.iter().filter(|t| t.certainty <= 40).count(),
        flow_sensitive: 0, // TODO
        cache_hit_rate: "94%".to_string(), // TODO
        analysis_speed: "125ms".to_string(), // TODO
    };

    let mut categories = HashMap::new();
    categories.insert("Platform".to_string(), CategoryDto { color: "#3498db".to_string(), icon: "🔧".to_string(), count: 1 });
    categories.insert("Configuration".to_string(), CategoryDto { color: "#e74c3c".to_string(), icon: "⚙️".to_string(), count: 1 });
    
    // Шаг 4: Создаем информацию о пагинации
    let total_items = all_types.len();
    let current_page = (offset / limit) + 1;
    let total_pages = (total_items + limit - 1) / limit; // Округление вверх
    let has_prev = current_page > 1;
    let has_next = current_page < total_pages;

    let pagination = Some(PaginationDto {
        current_page,
        page_size: limit,
        total_items,
        total_pages,
        has_prev,
        has_next,
    });

    // Шаг 5: Возвращаем полную структуру
    Json(AnalysisResultDto {
        types: type_dtos,
        categories, // TODO: Сгенерировать на основе реальных данных
        metrics,
        connections: Vec::new(), // TODO: Реализовать сбор связей
        pagination,
    })
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