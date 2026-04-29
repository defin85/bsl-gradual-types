use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context};
use bsl_analysis_v2::{
    AnalysisHostV2, AnalysisV2, Change as ChangeV2, FileId as V2FileId, SettingsId,
};
use bsl_backend::application::{
    CancellationPolicy, ExecutionContext, ExecutionSettings, IntellisenseV2Facade,
    ObservabilityOrigin, ObservabilityStage, PreparedOperationSnapshot, SemanticOperation,
};
use bsl_backend::system::{
    build_deps_bundle_v2_with_semantic_rules_config, global_runtime_config,
    load_semantic_rules_config_report, RuntimeKey, SemanticRulesConfig,
    SemanticRulesConfigDiagnostic, SystemCoordinator,
};
use bsl_shared::domain::types::{ParseError, TypeDiagnostic, TypeResolution};
use bsl_shared::domain::{TypeMetadataLookup, TypeResolver};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

pub(crate) struct CliPreparedFileOperation {
    pub(crate) context: ExecutionContext,
    pub(crate) coordinator: Arc<SystemCoordinator>,
    pub(crate) metadata_lookup: TypeMetadataLookup,
    pub(crate) resolver: Arc<TypeResolver>,
    pub(crate) _rules_config_path: Option<PathBuf>,
    pub(crate) _rules_config: SemanticRulesConfig,
    pub(crate) _rules_config_diagnostics: Vec<SemanticRulesConfigDiagnostic>,
    pub(crate) prepared: PreparedOperationSnapshot,
    pub(crate) file_id: V2FileId,
}

impl CliPreparedFileOperation {
    pub(crate) fn analysis(&self) -> &AnalysisV2 {
        &self.prepared.snapshot.analysis
    }

    pub(crate) fn index_snapshot(&self) -> &bsl_backend::system::IndexSnapshot {
        self.prepared.index_snapshot.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn file_path(&self) -> anyhow::Result<Arc<str>> {
        self.analysis()
            .file_path(self.file_id)
            .ok()
            .flatten()
            .context("cli runtime file_path unavailable")
    }

    pub(crate) fn ir_program(&self) -> anyhow::Result<Arc<SemanticProgram>> {
        IntellisenseV2Facade::run_ir_query_singleflight(
            &self.context,
            self.analysis(),
            Some(self.coordinator.as_ref()),
            self.file_id,
        )
        .map_err(|_| anyhow!("cli ir query cancelled"))?
        .context("cli ir unavailable")
    }

    pub(crate) fn syntax_diagnostics(&self) -> anyhow::Result<Arc<Vec<ParseError>>> {
        let diagnostics = IntellisenseV2Facade::run_optional_query(
            &self.context,
            ObservabilityStage::SyntaxDiagnosticsQuery,
            self.analysis(),
            Some(self.coordinator.as_ref()),
            |analysis| analysis.syntax_diagnostics(self.file_id),
        )
        .map_err(|_| anyhow!("cli syntax diagnostics cancelled"))?;

        Ok(diagnostics.unwrap_or_else(|| Arc::new(Vec::new())))
    }

    pub(crate) fn semantic_diagnostics(
        &self,
        include_flow_sensitive: bool,
    ) -> anyhow::Result<Arc<Vec<TypeDiagnostic>>> {
        let diagnostics = if include_flow_sensitive {
            IntellisenseV2Facade::run_optional_query(
                &self.context,
                ObservabilityStage::SemanticDiagnosticsQuery,
                self.analysis(),
                Some(self.coordinator.as_ref()),
                |analysis| analysis.semantic_diagnostics_flow_sensitive(self.file_id),
            )
            .map_err(|_| anyhow!("cli flow semantic diagnostics cancelled"))?
        } else {
            IntellisenseV2Facade::run_optional_query(
                &self.context,
                ObservabilityStage::SemanticDiagnosticsQuery,
                self.analysis(),
                Some(self.coordinator.as_ref()),
                |analysis| analysis.semantic_diagnostics(self.file_id),
            )
            .map_err(|_| anyhow!("cli semantic diagnostics cancelled"))?
        };

        Ok(diagnostics.unwrap_or_else(|| Arc::new(Vec::new())))
    }

    #[cfg(test)]
    pub(crate) fn exact_type_index_ready(&self) -> anyhow::Result<bool> {
        self.analysis()
            .current_type_index_serve_only_ready(self.file_id)
            .map_err(|_| anyhow!("cli exact type index readiness query cancelled"))
    }

    pub(crate) fn serve_only_type_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> anyhow::Result<Option<TypeResolution>> {
        self.analysis()
            .type_at_byte_offset_serve_only(self.file_id, byte_offset)
            .map_err(|_| anyhow!("cli serve-only type query cancelled"))
    }
}

pub(crate) fn cli_settings_id(diagnostics_detail_level: DetailLevel) -> SettingsId {
    SettingsId::from_hash(format!(
        "cli;schema={};diagnostics.detail_level={:?}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        diagnostics_detail_level
    ))
}

fn cli_operation_requires_exact_type_index(operation: SemanticOperation) -> bool {
    matches!(operation, SemanticOperation::Completion)
}

fn detect_cli_syntax_helper_path() -> Option<PathBuf> {
    global_runtime_config()
        .get_pathbuf(RuntimeKey::SyntaxHelperPath)
        .or_else(|| {
            [
                PathBuf::from("examples/syntax_helper"),
                PathBuf::from("../examples/syntax_helper"),
                PathBuf::from("C:/examples/syntax_helper"),
            ]
            .into_iter()
            .find(|path| path.exists())
        })
}

pub(crate) async fn prepare_cli_file_operation(
    path: &str,
    operation: SemanticOperation,
    diagnostics_detail_level: DetailLevel,
) -> anyhow::Result<CliPreparedFileOperation> {
    prepare_cli_file_operation_with_rules_config(path, operation, diagnostics_detail_level, None)
        .await
}

pub(crate) async fn prepare_cli_file_operation_with_rules_config(
    path: &str,
    operation: SemanticOperation,
    diagnostics_detail_level: DetailLevel,
    rules_config_override: Option<&str>,
) -> anyhow::Result<CliPreparedFileOperation> {
    let file_text = Arc::<str>::from(
        std::fs::read_to_string(path).with_context(|| format!("read CLI file {}", path))?,
    );
    let file_path = Arc::<str>::from(Path::new(path).to_string_lossy().into_owned());

    prepare_cli_text_operation_with_rules_config(
        file_text,
        file_path,
        operation,
        diagnostics_detail_level,
        rules_config_override,
    )
    .await
}

pub(crate) async fn prepare_cli_text_operation(
    file_text: Arc<str>,
    file_path: Arc<str>,
    operation: SemanticOperation,
    diagnostics_detail_level: DetailLevel,
) -> anyhow::Result<CliPreparedFileOperation> {
    prepare_cli_text_operation_with_rules_config(
        file_text,
        file_path,
        operation,
        diagnostics_detail_level,
        None,
    )
    .await
}

pub(crate) async fn prepare_cli_text_operation_with_rules_config(
    file_text: Arc<str>,
    file_path: Arc<str>,
    operation: SemanticOperation,
    diagnostics_detail_level: DetailLevel,
    rules_config_override: Option<&str>,
) -> anyhow::Result<CliPreparedFileOperation> {
    let coordinator = Arc::new(SystemCoordinator::new());
    let syntax_helper_path = detect_cli_syntax_helper_path();
    coordinator
        .start_with_paths(syntax_helper_path.as_deref(), None, None, None)
        .await?;

    let (rules_config_path, rules_config, rules_config_diagnostics) =
        load_cli_rules_config(Path::new(file_path.as_ref()), rules_config_override)?;

    let deps_bundle = build_deps_bundle_v2_with_semantic_rules_config(
        coordinator.as_ref(),
        syntax_helper_path.as_deref(),
        None,
        Some(&rules_config),
    )
    .context("build cli deps bundle")?;

    let settings = ExecutionSettings {
        settings_id: cli_settings_id(diagnostics_detail_level),
        diagnostics_detail_level,
    };

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });

    let facade =
        IntellisenseV2Facade::new(host, deps_bundle.index_snapshot, Some(coordinator.clone()));

    let deps = deps_bundle.semantic_deps;
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));

    let file_id = V2FileId(1);
    facade.apply_changes(vec![ChangeV2::SetFile {
        file_id,
        text: file_text,
        version: 0,
        path: file_path,
    }]);

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Runtime,
        operation,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(0),
        expected_deps_id: Some(deps_bundle.deps_id),
        flow_sensitive: false,
        settings,
        cancellation: CancellationPolicy::Ignore,
    };

    let prepared = facade
        .prepare_stateful_operation(&context, Some(coordinator.as_ref()))
        .await
        .map_err(|outcome| anyhow!("cli runtime preparation failed: {}", outcome.as_str()))?;

    if cli_operation_requires_exact_type_index(operation) {
        let version = prepared
            .snapshot
            .analysis
            .file_version(file_id)
            .map_err(|_| anyhow!("cli file version cancelled"))?
            .unwrap_or(0);

        prepared
            .snapshot
            .analysis
            .precompute_type_index_for_file(file_id, Some(version), 0)
            .map_err(|_| anyhow!("cli exact type index precompute cancelled"))?;

        let ready = prepared
            .snapshot
            .analysis
            .current_type_index_serve_only_ready(file_id)
            .map_err(|_| anyhow!("cli exact type index readiness cancelled"))?;
        if !ready {
            return Err(anyhow!(
                "cli exact type index unavailable after runtime preparation"
            ));
        }
    }

    Ok(CliPreparedFileOperation {
        context,
        coordinator,
        metadata_lookup,
        resolver,
        _rules_config_path: rules_config_path,
        _rules_config: rules_config,
        _rules_config_diagnostics: rules_config_diagnostics,
        prepared,
        file_id,
    })
}

fn load_cli_rules_config(
    file_path: &Path,
    rules_config_override: Option<&str>,
) -> anyhow::Result<(
    Option<PathBuf>,
    SemanticRulesConfig,
    Vec<SemanticRulesConfigDiagnostic>,
)> {
    let Some(path) = resolve_cli_rules_config_path(file_path, rules_config_override)? else {
        return Ok((None, SemanticRulesConfig::default(), Vec::new()));
    };

    let report = load_semantic_rules_config_report(Some(&path));
    for diagnostic in &report.diagnostics {
        eprintln!("warning: {}", diagnostic.message);
    }
    Ok((Some(path), report.config, report.diagnostics))
}

fn resolve_cli_rules_config_path(
    file_path: &Path,
    rules_config_override: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    if let Some(raw) = rules_config_override
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
    {
        let path = PathBuf::from(raw);
        return Ok(Some(if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .context("resolve current directory for --rules-config")?
                .join(path)
        }));
    }

    if let Some(found) = discover_bsl_rules_config(file_path.parent()) {
        return Ok(Some(found));
    }

    let cwd = std::env::current_dir().context("resolve current directory for rules discovery")?;
    Ok(discover_bsl_rules_config(Some(cwd.as_path())))
}

fn discover_bsl_rules_config(start: Option<&Path>) -> Option<PathBuf> {
    let mut current = start?;
    loop {
        let candidate = current.join("bsl-rules.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = current.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use bsl_shared::formatting::user_facing_resolution_type_name;
    use tempfile::TempDir;

    fn write_temp_module(content: &str) -> anyhow::Result<(TempDir, String)> {
        let temp = TempDir::new().context("tempdir")?;
        let path = temp.path().join("Module.bsl");
        fs::write(&path, content).context("write module")?;
        Ok((temp, path.to_string_lossy().to_string()))
    }

    #[tokio::test]
    async fn prepare_cli_file_operation_uses_runtime_contract_for_diagnostics() {
        let (_temp, path) = write_temp_module(
            "Процедура Test()\n    x = 1;\n    x.UnknownMethod();\nКонецПроцедуры\n",
        )
        .expect("temp module");

        let prepared =
            prepare_cli_file_operation(&path, SemanticOperation::Diagnostics, DetailLevel::Full)
                .await
                .expect("prepare cli diagnostics");

        assert_eq!(prepared.context.origin, ObservabilityOrigin::Runtime);
        assert_eq!(
            prepared.file_path().expect("file path").as_ref(),
            Path::new(&path).to_string_lossy()
        );

        let diagnostics = prepared
            .semantic_diagnostics(false)
            .expect("semantic diagnostics");
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("UnknownMethod")),
            "semantic diagnostics must come from the prepared shared runtime path: {diagnostics:#?}"
        );
    }

    #[tokio::test]
    async fn prepare_cli_file_operation_warms_exact_type_index_for_ir_queries() {
        let content = concat!(
            "Процедура Test()\n",
            "    Arr = Новый Массив;\n",
            "    ДляType = Arr;\n",
            "КонецПроцедуры\n"
        );
        let (_temp, path) = write_temp_module(content).expect("temp module");

        let prepared =
            prepare_cli_file_operation(&path, SemanticOperation::TypeAtPosition, DetailLevel::Full)
                .await
                .expect("prepare cli type-at-position");

        assert!(
            prepared
                .exact_type_index_ready()
                .expect("exact type index readiness"),
            "shared runtime preparation must warm the exact serve-only artifact for CLI type queries"
        );

        let ir = prepared.ir_program().expect("ir program");
        assert!(
            !ir.nodes.is_empty(),
            "IR query must succeed on the prepared shared runtime path"
        );

        let probe_offset = content.rfind("Arr").expect("Arr offset") as u32;
        let resolution = prepared
            .serve_only_type_at_byte_offset(probe_offset)
            .expect("serve-only type query")
            .expect("type resolution");
        assert!(
            user_facing_resolution_type_name(&resolution).starts_with("Массив"),
            "expected shared runtime array resolution, got {:?}",
            resolution
        );
    }

    #[test]
    fn cli_rules_config_discovers_bsl_rules_toml_from_file_parent() {
        let temp = TempDir::new().expect("tempdir");
        let module_dir = temp.path().join("CommonModules").join("Модуль").join("Ext");
        fs::create_dir_all(&module_dir).expect("module dir");
        let rules_path = temp.path().join("bsl-rules.toml");
        fs::write(&rules_path, "[semantic.common_module_factories]\n").expect("rules file");

        let discovered =
            resolve_cli_rules_config_path(&module_dir.join("Module.bsl"), None).expect("resolve");

        assert_eq!(discovered.as_deref(), Some(rules_path.as_path()));
    }

    #[test]
    fn cli_rules_config_override_parses_custom_rules_file() {
        let temp = TempDir::new().expect("tempdir");
        let rules_path = temp.path().join("custom-rules.toml");
        fs::write(
            &rules_path,
            r#"
[semantic.common_module_factories]
builtin_bsp = false

[[semantic.common_module_factories.rules]]
id = "cli-custom"
owner = "ОбщиеМодули.МойПомощник"
method = "ПолучитьМодуль"
argument_index = 0
target_mode = "common_module"
"#,
        )
        .expect("rules file");

        let (loaded_path, config, diagnostics) = load_cli_rules_config(
            Path::new("inline.bsl"),
            Some(rules_path.to_string_lossy().as_ref()),
        )
        .expect("load rules");

        assert_eq!(loaded_path.as_deref(), Some(rules_path.as_path()));
        assert!(diagnostics.is_empty());
        assert_eq!(
            config.identity.parse_status,
            bsl_backend::system::SemanticRulesConfigParseStatus::Parsed
        );
        assert!(config.identity.resolved_path.is_some());
        assert!(config.identity.content_hash.is_some());
        assert!(config
            .common_module_factories
            .find_rule("ОбщиеМодули.МойПомощник", "ПолучитьМодуль")
            .is_some());
        assert!(config
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .is_none());
    }

    #[test]
    fn cli_rules_config_malformed_uses_fail_closed_default_registry() {
        let temp = TempDir::new().expect("tempdir");
        let rules_path = temp.path().join("broken-rules.toml");
        fs::write(&rules_path, "not = [valid").expect("rules file");

        let (loaded_path, config, diagnostics) = load_cli_rules_config(
            Path::new("inline.bsl"),
            Some(rules_path.to_string_lossy().as_ref()),
        )
        .expect("load malformed rules fail-closed");

        assert_eq!(loaded_path.as_deref(), Some(rules_path.as_path()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            config.identity.parse_status,
            bsl_backend::system::SemanticRulesConfigParseStatus::Malformed
        );
        assert!(config.identity.resolved_path.is_some());
        assert!(config.identity.content_hash.is_none());
        assert!(config
            .common_module_factories
            .find_rule("ОбщиеМодули.ОбщегоНазначения", "ОбщийМодуль")
            .is_some());
    }
}
