//! bsl-repository
//!
//! Shared repository-related types that are used across adapters (HTTP/MCP/LSP) and core services.

/// Repository statistics used in API payloads and tooling.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
    pub last_update_time: Option<String>, // ISO 8601 timestamp
}
