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
mod tests {
    use super::{BslSettings, LspConfig};

    #[test]
    fn lsp_config_deserializes_feature_flags_from_initialization_options() {
        let raw = serde_json::json!({
            "platformDocsArchive": "/tmp/syntax_helper",
            "configurationPath": "/tmp/conf/Configuration.xml",
            "cacheEnabled": true,
            "strictFingerprint": false,
            "enableTypeHints": true,
            "enableCodeActions": false
        });

        let cfg: LspConfig = serde_json::from_value(raw).expect("LspConfig");
        assert_eq!(
            cfg.platform_docs_archive.as_deref(),
            Some("/tmp/syntax_helper")
        );
        assert_eq!(
            cfg.configuration_path.as_deref(),
            Some("/tmp/conf/Configuration.xml")
        );
        assert_eq!(cfg.cache_enabled, Some(true));
        assert_eq!(cfg.strict_fingerprint, Some(false));
        assert_eq!(cfg.enable_type_hints, Some(true));
        assert_eq!(cfg.enable_code_actions, Some(false));
    }

    #[test]
    fn bsl_settings_enable_flow_sensitive_defaults_to_false() {
        let raw = serde_json::json!({
            "hover": { "detailLevel": "full", "maxMethods": 10, "maxProperties": 5, "showCertainty": true },
            "formatting": { "enabled": false, "indentSize": 4 }
        });

        let settings: BslSettings = serde_json::from_value(raw).expect("BslSettings");
        assert!(!settings.enable_flow_sensitive);
    }

    #[test]
    fn bsl_settings_enable_flow_sensitive_deserializes_true() {
        let raw = serde_json::json!({
            "hover": { "detailLevel": "full", "maxMethods": 10, "maxProperties": 5, "showCertainty": true },
            "formatting": { "enabled": false, "indentSize": 4 },
            "enableFlowSensitive": true
        });

        let settings: BslSettings = serde_json::from_value(raw).expect("BslSettings");
        assert!(settings.enable_flow_sensitive);
    }
}

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
    /// Dev-only runtime overrides for `BSL_*` keys (applied only when `dev.enableDevEnvOverrides=true`).
    #[serde(default, rename = "devEnvOverrides")]
    pub dev_env_overrides: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub dev: DevSettings,
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
