//! Интеграционный тест на реальном файле из `examples/conf_big`:
//! локальная функция, объявленная ниже по файлу, должна резолвиться по return-типу.
//!
//! Регрессия: в BSL разрешён вызов функции до её объявления, и внутри модуля можно
//! вызывать неэкспортные методы.

mod support;

use std::path::PathBuf;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_shared::formatting::DetailLevel;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn conf_big_root() -> Option<PathBuf> {
    let candidates = [
        workspace_root().join("examples").join("conf_big"),
        PathBuf::from("examples/conf_big"),
        PathBuf::from("../examples/conf_big"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("Configuration.xml").exists())
}

#[test]
fn conf_big_local_function_call_resolves_return_type_before_declaration() {
    let Some(root) = conf_big_root() else {
        // Repo может поставляться без большой конфигурации — тогда тест пропускаем.
        return;
    };

    let module_rel = PathBuf::from("CommonModules")
        .join("АвансовыйОтчетФормы")
        .join("Ext")
        .join("Module.bsl");
    let module_path = root.join(&module_rel);
    assert!(
        module_path.exists(),
        "expected module to exist: {}",
        module_path.display()
    );

    let code = std::fs::read_to_string(&module_path).expect("read module file");
    let call_offset = code
        .find("ФункцияКотораяВозвращаетСтроку();")
        .expect("expected local function call in module") as u32;

    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let file_id = V2FileId(1);
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
        file_id,
        text: code.clone().into(),
        version: 0,
        path: module_rel.to_string_lossy().to_string().into(),
    });

    let analysis = host.analysis();
    let got = analysis
        .type_at_byte_offset(file_id, call_offset)
        .expect("type_at_byte_offset query")
        .map(|ty| ty.type_name());

    assert_eq!(
        got.as_deref(),
        Some("Строка"),
        "expected return type of local function call to be resolved"
    );
}

