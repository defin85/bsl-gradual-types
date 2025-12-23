//! Custom types and DTOs for BSL Language Server
//!
//! Contains request/response types for custom LSP commands.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;

// ============================================================================
// MILESTONE 2.20.3: Server Status notification
// ============================================================================

/// Custom bsl/serverStatus notification type
pub enum ServerStatus {}

impl Notification for ServerStatus {
    type Params = ServerStatusParams;
    const METHOD: &'static str = "bsl/serverStatus";
}

/// Parameters for bsl/serverStatus notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatusParams {
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ServerStatusParams {
    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            loading: true,
            message: Some(message.into()),
        }
    }

    pub fn ready() -> Self {
        Self {
            loading: false,
            message: None,
        }
    }
}

// ============================================================================
// Custom Request/Response Types (deprecated stubs)
// ============================================================================

/// Custom request: bsl/buildIndex - building type index
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BuildIndexParams {
    pub workspace_path: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct BuildIndexResponse {
    pub success: bool,
    pub types_count: usize,
    pub message: String,
}

/// Custom request: bsl/validateMethod - method call validation
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidateMethodParams {
    pub object_type: String,
    pub method_name: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ValidateMethodResponse {
    pub valid: bool,
    pub message: String,
}

/// Custom request: bsl/checkTypeCompatibility - type compatibility check
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CheckCompatibilityParams {
    pub source_type: String,
    pub target_type: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct CheckCompatibilityResponse {
    pub compatible: bool,
    pub message: String,
}

/// Custom request: bsl/incrementalUpdate - incremental index update
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IncrementalUpdateParams {
    pub config_path: String,
    pub platform_version: String,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub is_auto: bool,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct IncrementalUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// Custom request: bsl/pauseAutoReindex and bsl/resumeAutoReindex
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AutoReindexCommandParams {}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AutoReindexStateResponse {
    pub success: bool,
    pub paused: bool,
    pub message: String,
}

/// Custom request: bsl/extractPlatformDocs - platform documentation extraction
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExtractPlatformDocsParams {
    pub archive_path: String,
    pub platform_version: String,
    pub force: bool,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ExtractPlatformDocsResponse {
    pub success: bool,
    pub types_count: usize,
    pub message: String,
}

/// Custom request: bsl/renderTypeHtml - render HTML for type (uses TypeVisualization)
/// Reserved for future implementation
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RenderTypeHtmlParams {
    pub type_name: String,
    pub theme: Option<String>, // "light", "dark", "high-contrast"
}

/// Reserved for future implementation
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RenderTypeHtmlResponse {
    pub html: String,
    pub success: bool,
    pub message: Option<String>,
}

// ============================================================================
// GetCurrentContext types
// ============================================================================

/// Custom command: bsl.getCurrentContext - determine current function/procedure
#[derive(Debug, Deserialize)]
pub struct GetCurrentContextParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}
