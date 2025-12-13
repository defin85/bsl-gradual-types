//! Type Repository Statistics command handler
//!
//! MILESTONE 2.20.4: Handles bsl.getTypeRepositoryStats command.

use std::sync::Arc;
use tracing::debug;

use bsl_backend::system::SystemCoordinator;

/// Request for bsl.getTypeRepositoryStats (empty, no parameters)
/// Used by serde for deserialization even though fields are empty
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
pub struct GetTypeRepositoryStatsParams {}

/// Response for bsl.getTypeRepositoryStats
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeRepositoryStatsResponse {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update_time: Option<String>, // ISO 8601
}

/// Handle bsl.getTypeRepositoryStats command
pub fn handle_get_type_repository_stats(
    coordinator: Arc<SystemCoordinator>,
) -> TypeRepositoryStatsResponse {
    debug!("Handling bsl.getTypeRepositoryStats request");

    let stats = coordinator.get_type_repository_stats();

    debug!(
        "TypeRepository stats: total={}, platform={}, config={}",
        stats.total_types, stats.platform_types, stats.configuration_types
    );

    TypeRepositoryStatsResponse {
        total_types: stats.total_types,
        platform_types: stats.platform_types,
        configuration_types: stats.configuration_types,
        last_update_time: stats.last_update_time,
    }
}
