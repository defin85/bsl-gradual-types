//! Perf regression guard for IntelliSense v2 on a real conf_big module.
//! Verifies cold/warm query latency stays within coarse thresholds.

mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::{
    CancellationPolicy, ExecutionContext, ExecutionSettings, IntellisenseV2Facade,
    ObservabilityOrigin, ObservabilityStage, SemanticOperation,
};
use bsl_backend::system::{IndexSnapshot, IndexSnapshotId};
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

fn performance_limit_ms(debug_ms: u128, release_ms: u128) -> u128 {
    if cfg!(debug_assertions) {
        debug_ms
    } else {
        release_ms
    }
}

fn handle_missing_conf_big_fixture(reason: &str) {
    if std::env::var_os("CI").is_some() {
        panic!("{reason}");
    }
    eprintln!("skipping conf_big perf regression guard: {reason}");
}

async fn measure_shared_facade_semantic_triplet(
    runtime: &IntellisenseV2Facade,
    file_id: V2FileId,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    settings_id: SettingsId,
) -> std::time::Duration {
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: None,
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let started = Instant::now();
    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("prepare_stateful_operation");
    let analysis = prepared.snapshot.analysis;

    let ir_query = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        None,
        |analysis| analysis.ir(file_id),
    )
    .expect("ir query");

    let _ = IntellisenseV2Facade::run_parse_result_query(
        &context,
        &analysis,
        ir_query.is_some(),
        None,
        |analysis| analysis.parse_result(file_id),
    )
    .expect("parse_result query");

    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::SemanticDiagnosticsQuery,
        &analysis,
        None,
        |analysis| analysis.semantic_diagnostics(file_id),
    )
    .expect("semantic diagnostics query");

    started.elapsed()
}

#[tokio::test(flavor = "current_thread")]
async fn conf_big_module_cold_warm_perf_regression() {
    let Some(root) = conf_big_root() else {
        handle_missing_conf_big_fixture(
            "examples/conf_big fixture is missing (Configuration.xml not found)",
        );
        return;
    };

    let module_rel = PathBuf::from("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form")
        .join("Module.bsl");
    let module_path = root.join(&module_rel);
    if !module_path.exists() {
        handle_missing_conf_big_fixture(&format!(
            "module fixture is missing: {}",
            module_path.display()
        ));
        return;
    }

    let code = std::fs::read_to_string(&module_path).expect("read conf_big module");
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let file_id = V2FileId(1);
    let settings_id = SettingsId::from_hash("perf-regression");

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: code.clone().into(),
        version: 0,
        path: module_rel.to_string_lossy().to_string().into(),
    });

    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "perf-regression",
        ))),
        None,
    );

    let cold_elapsed = measure_shared_facade_semantic_triplet(
        &runtime,
        file_id,
        deps_bundle.deps_id.clone(),
        settings_id.clone(),
    )
    .await;
    let warm_elapsed = measure_shared_facade_semantic_triplet(
        &runtime,
        file_id,
        deps_bundle.deps_id.clone(),
        settings_id,
    )
    .await;

    println!(
        "conf_big module perf: cold={}ms warm={}ms path={}",
        cold_elapsed.as_millis(),
        warm_elapsed.as_millis(),
        module_rel.display()
    );

    let cold_limit = performance_limit_ms(60_000, 15_000);
    let warm_limit = performance_limit_ms(20_000, 6_000);

    assert!(
        cold_elapsed.as_millis() < cold_limit,
        "cold perf regression: {}ms >= {}ms",
        cold_elapsed.as_millis(),
        cold_limit
    );
    assert!(
        warm_elapsed.as_millis() < warm_limit,
        "warm perf regression: {}ms >= {}ms",
        warm_elapsed.as_millis(),
        warm_limit
    );
    assert!(
        warm_elapsed.as_millis() <= cold_elapsed.as_millis().saturating_mul(2),
        "warm pass unexpectedly much slower: cold={}ms warm={}ms",
        cold_elapsed.as_millis(),
        warm_elapsed.as_millis()
    );
}
