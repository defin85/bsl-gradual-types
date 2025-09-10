//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::TypeSystemService;
use bsl_shared::domain::types::{Certainty, ResolutionResult};

// --- DTOs for API ---
#[derive(Serialize, Clone)]
pub struct ApiMetrics {
    pub total_types: usize,
    pub known_types: usize,
    pub inferred_types: usize,
    pub unknown_types: usize,
}

#[derive(Serialize, Clone)]
pub struct ApiType {
    pub id: String,
    pub name: String,
    pub certainty: u8,
    pub category: String,
    pub source: String,
    pub facets: Vec<String>,
    pub union_types: Option<Vec<ApiUnionType>>,
}

#[derive(Serialize, Clone)]
pub struct ApiUnionType {
    pub type_name: String,
    pub weight: u8,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Clone)]
pub struct AppState {
    pub type_service: Arc<TypeSystemService>,
}

/// Get system metrics
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

    Json(ApiMetrics {
        total_types: all_types.len(),
        known_types: known,
        inferred_types: inferred,
        unknown_types: unknown,
    })
}

/// Get all types
pub async fn get_types(State(state): State<AppState>) -> impl IntoResponse {
    let all_types = state.type_service.get_all_platform_globals();

    let api_types: Vec<ApiType> = all_types
        .iter()
        .map(|(name, res)| {
            let (category, source) = match &res.source {
                bsl_shared::domain::types::ResolutionSource::Static => {
                    ("Platform".to_string(), "Static".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Inferred => {
                    ("Inferred".to_string(), "Inferred".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Annotated => {
                    ("Annotated".to_string(), "Annotated".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Runtime => {
                    ("Runtime".to_string(), "Runtime".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Predicted => {
                    ("Predicted".to_string(), "Predicted".to_string())
                }
            };

            let union_types = if let ResolutionResult::Union(types) = &res.result {
                Some(
                    types
                        .iter()
                        .map(|wt| ApiUnionType {
                            type_name: format!("{:?}", wt.type_),
                            weight: (wt.weight * 100.0) as u8,
                        })
                        .collect(),
                )
            } else {
                None
            };

            ApiType {
                id: name.clone(),
                name: name.clone(),
                certainty: match res.certainty {
                    Certainty::Known => 100,
                    Certainty::Inferred(val) => (val * 100.0) as u8,
                    Certainty::Unknown => 0,
                },
                category,
                source,
                facets: res
                    .available_facets
                    .iter()
                    .map(|f| format!("{:?}", f))
                    .collect(),
                union_types,
            }
        })
        .collect();

    Json(api_types)
}

/// Search types by query
pub async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types(&query.q).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}