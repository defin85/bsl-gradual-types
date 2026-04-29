use anyhow::{Context, Result};
use bsl_analysis_v2::semantic_rules::{
    normalized_rule_key, CommonModuleFactoryRegistry, CommonModuleFactoryRule,
    CommonModuleFactoryTargetMode,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const SEMANTIC_RULES_CONFIG_SCHEMA_VERSION: &str = "semantic-rules-v1";
pub const SEMANTIC_RULES_CONFIG_FILE_NAME: &str = "bsl-rules.toml";

#[derive(Debug, Clone)]
pub struct SemanticRulesConfig {
    pub common_module_factories: CommonModuleFactoryRegistry,
    pub identity: SemanticRulesConfigIdentity,
}

impl Default for SemanticRulesConfig {
    fn default() -> Self {
        let common_module_factories = CommonModuleFactoryRegistry::default();
        Self {
            identity: SemanticRulesConfigIdentity::new(
                None,
                None,
                SemanticRulesConfigParseStatus::Default,
                &common_module_factories,
            ),
            common_module_factories,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticRulesConfigIdentity {
    pub schema_version: &'static str,
    pub resolved_path: Option<String>,
    pub content_hash: Option<String>,
    pub parse_status: SemanticRulesConfigParseStatus,
    pub enabled_rules_hash: String,
}

impl SemanticRulesConfigIdentity {
    fn new(
        resolved_path: Option<String>,
        content_hash: Option<String>,
        parse_status: SemanticRulesConfigParseStatus,
        common_module_factories: &CommonModuleFactoryRegistry,
    ) -> Self {
        Self {
            schema_version: SEMANTIC_RULES_CONFIG_SCHEMA_VERSION,
            resolved_path,
            content_hash,
            parse_status,
            enabled_rules_hash: common_module_factory_registry_hash(common_module_factories),
        }
    }

    pub fn cache_key_payload(&self) -> String {
        format!(
            "schema={};path={};content_hash={};parse_status={};enabled_rules_hash={}",
            self.schema_version,
            self.resolved_path.as_deref().unwrap_or("none"),
            self.content_hash.as_deref().unwrap_or("none"),
            self.parse_status.as_str(),
            self.enabled_rules_hash
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRulesConfigParseStatus {
    Default,
    Parsed,
    Missing,
    Malformed,
}

impl SemanticRulesConfigParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Parsed => "parsed",
            Self::Missing => "missing",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemanticRulesConfigDiagnostic {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SemanticRulesConfigLoadReport {
    pub config: SemanticRulesConfig,
    pub diagnostics: Vec<SemanticRulesConfigDiagnostic>,
}

pub fn parse_semantic_rules_config_toml(content: &str) -> Result<SemanticRulesConfig> {
    parse_semantic_rules_config_toml_with_identity(content, None, None)
}

fn parse_semantic_rules_config_toml_with_identity(
    content: &str,
    resolved_path: Option<String>,
    content_hash: Option<String>,
) -> Result<SemanticRulesConfig> {
    let raw: RawRulesConfig =
        toml::from_str(content).context("parse bsl-rules.toml semantic rules config")?;
    Ok(raw.into_config(resolved_path, content_hash))
}

pub fn load_semantic_rules_config(path: Option<&Path>) -> Result<SemanticRulesConfig> {
    Ok(load_semantic_rules_config_report(path).config)
}

pub fn resolve_semantic_rules_config_path(
    explicit_rules_config: Option<&Path>,
    default_start: Option<&Path>,
) -> Option<PathBuf> {
    explicit_rules_config
        .map(normalize_rules_path_best_effort)
        .or_else(|| discover_semantic_rules_config(default_start))
}

pub fn discover_semantic_rules_config(start: Option<&Path>) -> Option<PathBuf> {
    let start = start?;
    let mut current = if start.is_file() {
        start.parent()?
    } else {
        start
    };

    loop {
        let candidate = current.join(SEMANTIC_RULES_CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(normalize_rules_path_best_effort(candidate.as_path()));
        }
        current = current.parent()?;
    }
}

pub fn load_semantic_rules_config_report(path: Option<&Path>) -> SemanticRulesConfigLoadReport {
    let Some(path) = path else {
        return SemanticRulesConfigLoadReport {
            config: SemanticRulesConfig::default(),
            diagnostics: Vec::new(),
        };
    };

    let resolved_path = resolve_rules_path(path);
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            let mut config = SemanticRulesConfig::default();
            config.identity.resolved_path = Some(resolved_path);
            config.identity.parse_status = SemanticRulesConfigParseStatus::Missing;
            return SemanticRulesConfigLoadReport {
                config,
                diagnostics: vec![SemanticRulesConfigDiagnostic {
                    message: format!(
                        "Failed to read semantic rules config {}: {err}",
                        path.display()
                    ),
                }],
            };
        }
    };
    let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    match parse_semantic_rules_config_toml_with_identity(
        &content,
        Some(resolved_path.clone()),
        Some(content_hash.clone()),
    ) {
        Ok(config) => SemanticRulesConfigLoadReport {
            config,
            diagnostics: Vec::new(),
        },
        Err(err) => {
            let mut config = SemanticRulesConfig::default();
            config.identity.resolved_path = Some(resolved_path);
            config.identity.content_hash = Some(content_hash);
            config.identity.parse_status = SemanticRulesConfigParseStatus::Malformed;
            SemanticRulesConfigLoadReport {
                config,
                diagnostics: vec![SemanticRulesConfigDiagnostic {
                    message: format!("Malformed semantic rules config {}: {err}", path.display()),
                }],
            }
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawRulesConfig {
    #[serde(default)]
    semantic: RawSemanticSection,
}

#[derive(Debug, Deserialize, Default)]
struct RawSemanticSection {
    #[serde(default)]
    common_module_factories: RawCommonModuleFactoriesSection,
}

#[derive(Debug, Deserialize)]
struct RawCommonModuleFactoriesSection {
    #[serde(default = "default_true")]
    builtin_bsp: bool,
    #[serde(default)]
    rules: Vec<RawCommonModuleFactoryRule>,
}

impl Default for RawCommonModuleFactoriesSection {
    fn default() -> Self {
        Self {
            builtin_bsp: true,
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawCommonModuleFactoryRule {
    id: String,
    owner: String,
    method: String,
    argument_index: usize,
    target_mode: RawCommonModuleFactoryTargetMode,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawCommonModuleFactoryTargetMode {
    CommonModule,
    CommonModuleOrManager,
}

impl RawRulesConfig {
    fn into_config(
        self,
        resolved_path: Option<String>,
        content_hash: Option<String>,
    ) -> SemanticRulesConfig {
        let factories = self.semantic.common_module_factories;
        let mut rules = Vec::new();
        if factories.builtin_bsp {
            rules.push(CommonModuleFactoryRule::builtin_bsp_common_module());
        }
        for rule in factories
            .rules
            .into_iter()
            .map(CommonModuleFactoryRule::from)
        {
            apply_common_module_factory_override(&mut rules, rule);
        }
        let common_module_factories = CommonModuleFactoryRegistry::new(rules);

        SemanticRulesConfig {
            identity: SemanticRulesConfigIdentity::new(
                resolved_path,
                content_hash,
                SemanticRulesConfigParseStatus::Parsed,
                &common_module_factories,
            ),
            common_module_factories,
        }
    }
}

fn apply_common_module_factory_override(
    rules: &mut Vec<CommonModuleFactoryRule>,
    override_rule: CommonModuleFactoryRule,
) {
    let override_id = normalized_rule_key(&override_rule.id);
    if let Some(existing_index) = rules
        .iter()
        .position(|rule| normalized_rule_key(&rule.id) == override_id)
    {
        if override_rule.enabled {
            rules[existing_index] = override_rule;
        } else {
            rules.remove(existing_index);
        }
        return;
    }

    if override_rule.enabled {
        rules.push(override_rule);
    }
}

impl From<RawCommonModuleFactoryRule> for CommonModuleFactoryRule {
    fn from(raw: RawCommonModuleFactoryRule) -> Self {
        Self {
            id: raw.id,
            owner: raw.owner,
            method: raw.method,
            argument_index: raw.argument_index,
            target_mode: raw.target_mode.into(),
            enabled: raw.enabled,
        }
    }
}

impl From<RawCommonModuleFactoryTargetMode> for CommonModuleFactoryTargetMode {
    fn from(raw: RawCommonModuleFactoryTargetMode) -> Self {
        match raw {
            RawCommonModuleFactoryTargetMode::CommonModule => Self::CommonModule,
            RawCommonModuleFactoryTargetMode::CommonModuleOrManager => Self::CommonModuleOrManager,
        }
    }
}

fn default_true() -> bool {
    true
}

fn resolve_rules_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn normalize_rules_path_best_effort(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn common_module_factory_registry_hash(registry: &CommonModuleFactoryRegistry) -> String {
    let mut enabled_rules: Vec<String> = registry
        .rules()
        .iter()
        .filter(|rule| rule.enabled)
        .map(|rule| {
            format!(
                "id={};owner={};method={};argument_index={};target_mode={}",
                rule.id.trim(),
                rule.owner.trim().to_lowercase(),
                rule.method.trim().to_lowercase(),
                rule.argument_index,
                common_module_factory_target_mode_key(rule.target_mode)
            )
        })
        .collect();
    enabled_rules.sort();
    blake3::hash(enabled_rules.join("\n").as_bytes())
        .to_hex()
        .to_string()
}

fn common_module_factory_target_mode_key(mode: CommonModuleFactoryTargetMode) -> &'static str {
    match mode {
        CommonModuleFactoryTargetMode::CommonModule => "common_module",
        CommonModuleFactoryTargetMode::CommonModuleOrManager => "common_module_or_manager",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rules_config_keeps_builtin_bsp_factory_enabled() {
        let parsed = parse_semantic_rules_config_toml("").expect("parse empty config");
        let rule = parsed
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .expect("builtin BSP rule");

        assert_eq!(rule.argument_index, 0);
        assert_eq!(
            rule.target_mode,
            CommonModuleFactoryTargetMode::CommonModuleOrManager
        );
        assert_eq!(
            parsed.identity.parse_status,
            SemanticRulesConfigParseStatus::Parsed
        );
        assert!(!parsed.identity.enabled_rules_hash.is_empty());
    }

    #[test]
    fn parses_common_module_factory_rules_and_builtin_toggle() {
        let parsed = parse_semantic_rules_config_toml(
            r#"
[semantic.common_module_factories]
builtin_bsp = false

[[semantic.common_module_factories.rules]]
id = "custom-factory"
owner = "ОбщиеМодули.МойПомощник"
method = "ПолучитьМодуль"
argument_index = 1
target_mode = "common_module"
enabled = true
"#,
        )
        .expect("parse custom rule");

        assert!(parsed
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .is_none());

        let custom = parsed
            .common_module_factories
            .find_rule("ОбщиеМодули.МойПомощник", "ПолучитьМодуль")
            .expect("custom rule");
        assert_eq!(custom.id, "custom-factory");
        assert_eq!(custom.argument_index, 1);
        assert_eq!(
            custom.target_mode,
            CommonModuleFactoryTargetMode::CommonModule
        );
    }

    #[test]
    fn project_rule_can_disable_builtin_by_same_id() {
        let parsed = parse_semantic_rules_config_toml(
            r#"
[semantic.common_module_factories]
builtin_bsp = true

[[semantic.common_module_factories.rules]]
id = "bsp-common-purpose-common-module"
owner = "ОбщиеМодули.ОбщегоНазначения"
method = "ОбщийМодуль"
argument_index = 0
target_mode = "common_module_or_manager"
enabled = false
"#,
        )
        .expect("parse disabled builtin override");

        assert!(parsed
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .is_none());
        assert!(!parsed
            .common_module_factories
            .rules()
            .iter()
            .any(|rule| rule.id == "bsp-common-purpose-common-module"));
    }

    #[test]
    fn rejects_unknown_common_module_factory_target_mode() {
        let err = parse_semantic_rules_config_toml(
            r#"
[semantic.common_module_factories]

[[semantic.common_module_factories.rules]]
id = "bad"
owner = "ОбщиеМодули.МойПомощник"
method = "ПолучитьМодуль"
argument_index = 0
target_mode = "manager"
"#,
        )
        .expect_err("unknown target mode must fail schema parsing");

        assert!(err.to_string().contains("parse bsl-rules.toml"));
    }

    #[test]
    fn malformed_rules_config_load_report_uses_default_registry_with_diagnostic() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        let malformed_content = "not = [valid";
        std::fs::write(temp.path(), malformed_content).expect("write malformed rules");

        let report = load_semantic_rules_config_report(Some(temp.path()));

        assert_eq!(
            report.config.identity.parse_status,
            SemanticRulesConfigParseStatus::Malformed
        );
        assert_eq!(
            report.config.identity.content_hash.as_deref(),
            Some(blake3::hash(malformed_content.as_bytes()).to_hex().as_str())
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report
            .config
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .is_some());
    }
}
