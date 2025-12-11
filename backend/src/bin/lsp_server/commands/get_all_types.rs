//! Get All Types command handler
//!
//! Handles bsl.getAllTypes custom command for TreeView.

use std::sync::Arc;
use tracing::info;

use bsl_backend::application::TypeSystemService;
use bsl_shared::api::dtos::AnalysisResultDto;

/// Request for bsl.getAllTypes
#[derive(Debug, serde::Deserialize)]
pub struct GetAllTypesRequest {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub category: Option<String>,
}

fn default_limit() -> usize {
    1000
}

/// Handle bsl.getAllTypes command
pub fn handle_get_all_types(
    params: GetAllTypesRequest,
    type_system_service: Option<Arc<TypeSystemService>>,
) -> AnalysisResultDto {
    info!(
        "Custom command: bsl.getAllTypes - limit: {}, offset: {}, category: {:?}",
        params.limit, params.offset, params.category
    );

    match type_system_service {
        Some(service) => service.get_all_types_as_dto(
            params.limit,
            params.offset,
            params.category,
            None,  // certainty_filter
            false, // flow_sensitive_only
        ),
        None => {
            tracing::warn!("TypeSystemService not available");
            AnalysisResultDto {
                types: vec![],
                categories: std::collections::HashMap::new(),
                metrics: bsl_shared::api::dtos::MetricsDto {
                    total_types: 0,
                    certainty_high: 0,
                    certainty_medium: 0,
                    certainty_low: 0,
                    flow_sensitive: 0,
                    cache_hit_rate: "0%".to_string(),
                    analysis_speed: "N/A".to_string(),
                },
                connections: vec![],
                pagination: None,
            }
        }
    }
}
