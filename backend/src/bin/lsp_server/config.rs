//! Configuration types for BSL Language Server
//!
//! Contains LSP initialization options and workspace settings.

use serde::Deserialize;

/// LSP Configuration - passed from VSCode Extension through initializationOptions
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspConfig {
    /// Path to parent folder with 1C platform documentation (syntax_helper)
    /// Must contain subfolders: rebuilt.shcntx_ru and rebuilt.shlang_ru
    pub platform_docs_archive: Option<String>,

    /// Path to Configuration.xml of 1C configuration
    pub configuration_path: Option<String>,

    /// 1C platform version (e.g., "8.3.25")
    /// Reserved for future versioned type loading
    #[allow(dead_code)]
    pub platform_version: Option<String>,
}

/// MILESTONE 3.6 Phase 1+3: BSL Settings (from workspace/didChangeConfiguration)
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BslSettings {
    pub hover: HoverSettings,
    #[serde(default)]
    pub diagnostics: DiagnosticsSettings,
}

/// MILESTONE 3.6 Phase 1: Hover Settings
#[derive(Debug, Clone, Deserialize)]
pub struct HoverSettings {
    #[serde(rename = "detailLevel")]
    pub detail_level: String, // "compact" | "full" | "detailed"

    #[serde(rename = "maxMethods")]
    pub max_methods: usize,

    #[serde(rename = "maxProperties")]
    pub max_properties: usize,

    #[serde(rename = "showCertainty")]
    pub show_certainty: bool,
}

impl Default for HoverSettings {
    fn default() -> Self {
        Self {
            detail_level: "full".to_string(),
            max_methods: 10,
            max_properties: 5,
            show_certainty: true,
        }
    }
}

/// MILESTONE 3.6 Phase 3: Diagnostics Settings
#[derive(Debug, Clone, Deserialize)]
pub struct DiagnosticsSettings {
    #[serde(rename = "detailLevel")]
    pub detail_level: String, // "brief" | "standard" | "detailed"

    /// Reserved for future use - will control hint-level diagnostics
    #[serde(rename = "showHints")]
    #[allow(dead_code)]
    pub show_hints: bool,
}

impl Default for DiagnosticsSettings {
    fn default() -> Self {
        Self {
            detail_level: "standard".to_string(),
            show_hints: true,
        }
    }
}
