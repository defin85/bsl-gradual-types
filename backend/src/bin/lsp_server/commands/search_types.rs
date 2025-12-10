//! Search Types command handler
//!
//! Handles bsl.searchTypes custom command.

use std::sync::Arc;
use tracing::{info, warn};

use bsl_shared::engine::AnalysisEngine;

/// Request for bsl.searchTypes
#[derive(Debug, serde::Deserialize)]
pub struct SearchTypesRequest {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    15
}

/// Response for bsl.searchTypes
#[derive(Debug, serde::Serialize)]
pub struct SearchTypesResponse {
    pub types: Vec<TypeSearchResult>,
    pub total: usize,
}

/// Single search result
#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeSearchResult {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub english_name: Option<String>,
    pub facet: String,
    pub certainty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Handle bsl.searchTypes command
pub fn handle_search_types(
    params: SearchTypesRequest,
    analysis_engine: Option<Arc<AnalysisEngine>>,
) -> SearchTypesResponse {
    info!(
        "Custom command: bsl.searchTypes - query: '{}', limit: {}",
        params.query, params.limit
    );

    let engine = match analysis_engine {
        Some(e) => e,
        None => {
            warn!("AnalysisEngine not available");
            return SearchTypesResponse {
                types: vec![],
                total: 0,
            };
        }
    };

    let repo = engine.get_repository();
    let all_types = repo.get_all_types();

    if all_types.is_empty() {
        warn!("TypeRepository is empty - platform types not loaded yet");
        return SearchTypesResponse {
            types: vec![],
            total: 0,
        };
    }

    // Filter by query (case-insensitive partial match)
    let query_lower = params.query.to_lowercase();
    let filtered: Vec<TypeSearchResult> = all_types
        .iter()
        .filter(|t| {
            t.name.to_lowercase().contains(&query_lower)
                || (!t.english_name.is_empty()
                    && t.english_name.to_lowercase().contains(&query_lower))
        })
        .take(params.limit)
        .map(|t| {
            let facet = if t.facets.is_empty() {
                "Object".to_string()
            } else {
                format!("{:?}", t.facets[0])
            };

            TypeSearchResult {
                name: t.name.clone(),
                english_name: if t.english_name.is_empty() {
                    None
                } else {
                    Some(t.english_name.clone())
                },
                facet,
                certainty: "Known (100%)".to_string(),
                description: if t.description.is_empty() {
                    None
                } else {
                    Some(t.description.clone())
                },
            }
        })
        .collect();

    let total = filtered.len();
    info!("Found {} types matching '{}'", total, params.query);

    SearchTypesResponse {
        types: filtered,
        total,
    }
}
