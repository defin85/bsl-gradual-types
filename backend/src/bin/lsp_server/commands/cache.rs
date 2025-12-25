use serde::Deserialize;
use std::sync::Arc;

use bsl_backend::system::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, SystemCoordinator,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheCommandParams {
    pub configuration_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheToggleParams {
    pub enabled: bool,
}

pub async fn handle_cache_stats(
    coordinator: Arc<SystemCoordinator>,
    scope: CacheScope,
) -> Result<CacheStatsReport, String> {
    coordinator
        .cache_stats(&scope)
        .await
        .map_err(|e| e.to_string())
}

pub async fn handle_cache_clear(
    coordinator: Arc<SystemCoordinator>,
    scope: CacheScope,
) -> Result<CacheClearReport, String> {
    coordinator
        .clear_cache_scope(&scope)
        .await
        .map_err(|e| e.to_string())
}

pub async fn handle_cache_set_enabled(
    coordinator: Arc<SystemCoordinator>,
    enabled: bool,
) -> Result<CacheToggleResult, String> {
    Ok(coordinator.set_cache_enabled(enabled).await)
}
