//! Configuration types for BSL Language Server
//!
//! Contains LSP initialization options and workspace settings.

use std::collections::HashMap;

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

    /// Path to `bsl-rules.toml` semantic rules configuration.
    pub rules_config: Option<String>,

    /// 1C platform version (e.g., "8.3.25")
    /// Reserved for future versioned type loading
    #[allow(dead_code)]
    pub platform_version: Option<String>,

    /// Enable/disable cache (workspace setting)
    #[serde(default)]
    pub cache_enabled: Option<bool>,

    /// Use strict fingerprint mode for cache keys and deps snapshots
    #[serde(default)]
    pub strict_fingerprint: Option<bool>,

    /// Feature gate: enable LSP inlay hints (textDocument/inlayHint) if supported by server.
    ///
    /// Passed from VS Code extension via initializationOptions.enableTypeHints.
    #[serde(default)]
    pub enable_type_hints: Option<bool>,

    /// Feature gate: enable LSP code actions (textDocument/codeAction) if supported by server.
    ///
    /// Passed from VS Code extension via initializationOptions.enableCodeActions.
    #[serde(default)]
    pub enable_code_actions: Option<bool>,
}

/// VS Code "bsl.typeHints.*" settings (from workspace/didChangeConfiguration, section `bsl`)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeHintsSettings {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_true")]
    pub show_variable_types: bool,

    #[serde(default = "default_true")]
    #[allow(dead_code)]
    pub show_return_types: bool,

    #[serde(default = "default_true")]
    pub show_union_details: bool,

    #[serde(default = "default_min_certainty")]
    pub min_certainty: f64,
}

impl Default for TypeHintsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            show_variable_types: true,
            show_return_types: true,
            show_union_details: true,
            min_certainty: default_min_certainty(),
        }
    }
}

/// VS Code "bsl.codeActions.*" settings (from workspace/didChangeConfiguration, section `bsl`)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionsSettings {
    #[serde(default)]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_min_certainty() -> f64 {
    0.7
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

/// MILESTONE 3.6 Phase 1+3: BSL Settings (from workspace/didChangeConfiguration)
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BslSettings {
    pub hover: HoverSettings,
    #[serde(default)]
    pub diagnostics: DiagnosticsSettings,
    #[serde(default)]
    pub formatting: FormattingSettings,
    #[serde(default, rename = "typeHints")]
    pub type_hints: TypeHintsSettings,
    #[serde(default, rename = "codeActions")]
    pub code_actions: CodeActionsSettings,
    /// Workspace setting `enableFlowSensitive` (default: false).
    #[serde(default, rename = "enableFlowSensitive")]
    pub enable_flow_sensitive: bool,
    /// Stable runtime overrides for `BSL_*` keys (see bsl-runtime runtime_config registry).
    #[serde(default, rename = "envOverrides")]
    pub env_overrides: HashMap<String, serde_json::Value>,
    /// Dev-only runtime overrides for `BSL_*` keys (applied only when effective gate is true).
    #[serde(default, rename = "devEnvOverrides")]
    pub dev_env_overrides: HashMap<String, serde_json::Value>,
    /// Canonical gate for dev-only runtime overrides used by both LSP and bsl-agent payloads.
    ///
    /// If absent, legacy `dev.enableDevEnvOverrides` is used for backward compatibility.
    #[serde(default, rename = "allowDevOverrides")]
    pub allow_dev_overrides: Option<bool>,
    #[serde(default)]
    pub dev: DevSettings,
}

impl BslSettings {
    /// Effective gate for dev-only runtime overrides.
    ///
    /// Canonical `allowDevOverrides` takes precedence; legacy `dev.enableDevEnvOverrides`
    /// is used only as fallback for backward compatibility.
    pub fn enable_dev_env_overrides(&self) -> bool {
        self.allow_dev_overrides
            .unwrap_or(self.dev.enable_dev_env_overrides)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DevSettings {
    #[serde(default)]
    pub enable_dev_env_overrides: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormattingSettings {
    #[serde(rename = "enabled")]
    pub enabled: bool,

    #[serde(rename = "indentSize")]
    pub indent_size: usize,
}

impl Default for FormattingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            indent_size: 4,
        }
    }
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
