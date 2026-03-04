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

#[test]
fn bsl_settings_allow_dev_overrides_uses_canonical_when_present() {
    let raw = serde_json::json!({
        "hover": { "detailLevel": "full", "maxMethods": 10, "maxProperties": 5, "showCertainty": true },
        "formatting": { "enabled": false, "indentSize": 4 },
        "allowDevOverrides": false,
        "dev": { "enableDevEnvOverrides": true }
    });

    let settings: BslSettings = serde_json::from_value(raw).expect("BslSettings");
    assert_eq!(settings.allow_dev_overrides, Some(false));
    assert!(!settings.enable_dev_env_overrides());
}

#[test]
fn bsl_settings_allow_dev_overrides_falls_back_to_legacy_flag() {
    let raw = serde_json::json!({
        "hover": { "detailLevel": "full", "maxMethods": 10, "maxProperties": 5, "showCertainty": true },
        "formatting": { "enabled": false, "indentSize": 4 },
        "dev": { "enableDevEnvOverrides": true }
    });

    let settings: BslSettings = serde_json::from_value(raw).expect("BslSettings");
    assert_eq!(settings.allow_dev_overrides, None);
    assert!(settings.enable_dev_env_overrides());
}
