#![allow(dead_code)]

use std::path::Path;
use std::sync::{Arc, LazyLock};

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_hover_info_with_semantic_program;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use bsl_backend::system::{DepsBundleV2, SystemCoordinator, build_deps_bundle_v2};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{ParseError, TypeDiagnostic};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

pub fn deps_bundle_v2_for_paths(
    syntax_helper_path: Option<&Path>,
    config_path: Option<&Path>,
    platform_version: Option<&str>,
) -> Arc<DepsBundleV2> {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(syntax_helper_path, config_path, platform_version, None)
        .expect("startup");

    let deps_bundle =
        build_deps_bundle_v2(&coordinator, syntax_helper_path, config_path).expect("deps bundle v2");

    Arc::new(deps_bundle)
}

pub fn deps_bundle_v2_fallback() -> Arc<DepsBundleV2> {
    static BUNDLE: LazyLock<Arc<DepsBundleV2>> =
        LazyLock::new(|| deps_bundle_v2_for_paths(None, None, None));
    BUNDLE.clone()
}

pub fn deps_bundle_v2_with_syntax_helper() -> Arc<DepsBundleV2> {
    static BUNDLE: LazyLock<Arc<DepsBundleV2>> = LazyLock::new(|| {
        deps_bundle_v2_for_paths(Some(Path::new("examples/syntax_helper")), None, None)
    });
    BUNDLE.clone()
}

pub fn ir_program_for_code(deps_bundle: &DepsBundleV2, file_path: &str, code: &str) -> Arc<SemanticProgram> {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("tests"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    analysis
        .ir(V2FileId(1))
        .map_err(|_| anyhow::anyhow!("ir query cancelled"))
        .and_then(|value| value.ok_or_else(|| anyhow::anyhow!("ir unavailable")))
        .expect("ir")
}

pub fn syntax_diagnostics_for_code(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
) -> Vec<ParseError> {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("tests"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    analysis
        .syntax_diagnostics(V2FileId(1))
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default()
}

pub fn semantic_diagnostics_for_code(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
) -> Vec<TypeDiagnostic> {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("tests"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    analysis
        .semantic_diagnostics(V2FileId(1))
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default()
}

pub fn hover_for_code(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
    line: u32,
    column: u32,
) -> Option<String> {
    hover_for_code_with_config(deps_bundle, file_path, code, line, column, None)
}

pub fn hover_for_code_with_config(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
    line: u32,
    column: u32,
    hover_config: Option<HoverFormatConfig>,
) -> Option<String> {
    let ir_program = ir_program_for_code(deps_bundle, file_path, code);

    let deps = deps_bundle.semantic_deps.clone();
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let hover_formatter = HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup.clone());

    get_hover_info_with_semantic_program(
        code,
        line,
        column,
        &metadata_lookup,
        &hover_formatter,
        hover_config,
        resolver.as_ref(),
        ir_program,
    )
}
