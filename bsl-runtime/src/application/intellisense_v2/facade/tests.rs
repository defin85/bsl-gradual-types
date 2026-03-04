use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;

#[tokio::test]
async fn p7_apply_changes_and_wait_for_version_works() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(1);

    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("abc"),
        version: 7,
        path: Arc::from("test.bsl"),
    }]);

    let ok = timeout(
        Duration::from_secs(1),
        runtime.wait_for_file_version(file_id, 7),
    )
    .await
    .expect("wait_for_file_version timeout");
    assert!(ok, "expected wait_for_file_version to succeed");

    let analysis = runtime.snapshot().await;
    assert_eq!(analysis.file_version(file_id).unwrap(), Some(7));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn p7_waiters_are_released_on_shutdown() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let file_id = FileId(1);

    let wait_task = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.wait_for_file_version(file_id, 42).await }
    });

    runtime.shutdown_for_test().await;

    let ok = timeout(Duration::from_secs(1), wait_task)
        .await
        .expect("wait task timeout")
        .expect("wait task join");
    assert!(!ok, "expected waiter to return false on shutdown");
}

#[tokio::test]
async fn interactive_commands_preempt_background_backlog() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );
    let mut sleepers = Vec::new();
    for _ in 0..6 {
        sleepers.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Background, Duration::from_millis(40)),
        );
    }

    let started = Instant::now();
    let interactive_ack = runtime.enqueue_test_noop(RuntimeQueuePriority::Interactive);
    timeout(Duration::from_millis(120), interactive_ack)
        .await
        .expect("interactive noop must not wait for full background backlog")
        .expect("interactive noop ack");
    assert!(
        started.elapsed() < Duration::from_millis(120),
        "interactive noop should complete before background backlog drains"
    );

    for sleeper_ack in sleepers {
        timeout(Duration::from_secs(1), sleeper_ack)
            .await
            .expect("background sleeper ack timeout")
            .expect("background sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn background_commands_make_progress_under_interactive_flood() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p7"))),
        None,
    );

    let mut interactive_sleep_acks = Vec::new();
    for _ in 0..100 {
        interactive_sleep_acks.push(
            runtime.enqueue_test_sleep(RuntimeQueuePriority::Interactive, Duration::from_millis(5)),
        );
    }
    let background_ack = runtime.enqueue_test_noop(RuntimeQueuePriority::Background);
    timeout(Duration::from_millis(200), background_ack)
        .await
        .expect("background command should make progress despite interactive flood")
        .expect("background noop ack");

    for interactive_ack in interactive_sleep_acks {
        timeout(Duration::from_secs(2), interactive_ack)
            .await
            .expect("interactive sleeper ack timeout")
            .expect("interactive sleeper ack");
    }

    runtime.shutdown_for_test().await;
}

fn make_deps() -> Arc<SemanticDeps> {
    let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
    let signature_index = repository.get_signature_index_clone();
    let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));
    let platform_signatures_loaded = repository.platform_docs_loaded();
    Arc::new(SemanticDeps {
        repository,
        signature_index,
        resolver,
        platform_signatures_loaded,
    })
}

fn make_index_snapshot(raw_id: &str) -> Arc<IndexSnapshot> {
    Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(raw_id)))
}

#[tokio::test]
async fn p8_snapshot_with_deps_is_atomic() {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    {
        let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
        assert_eq!(deps_id.as_str(), "deps_old");
        assert_eq!(index_snapshot.id.as_str(), "index_old");
        assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");
    }

    let deps_new = make_deps();
    let deps_id_new = DepsSnapshotId::from_hash("deps_new");
    let index_new = make_index_snapshot("index_new");

    let apply_task = tokio::spawn({
        let runtime = runtime.clone();
        let deps_new = deps_new.clone();
        let deps_id_new = deps_id_new.clone();
        let index_new = index_new.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let ok = runtime
                .apply_deps_bundle(deps_id_new, deps_new, index_new)
                .await;
            assert!(ok, "apply_deps_bundle should succeed");
        }
    });

    let watch_task = tokio::spawn({
        let runtime = runtime.clone();
        async move {
            for _ in 0..200 {
                let (_analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
                match deps_id.as_str() {
                    "deps_old" => assert_eq!(index_snapshot.id.as_str(), "index_old"),
                    "deps_new" => assert_eq!(index_snapshot.id.as_str(), "index_new"),
                    other => panic!("unexpected deps_id: {}", other),
                }
            }
        }
    });

    apply_task.await.expect("apply task join");
    watch_task.await.expect("watch task join");

    let (analysis, index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
    assert_eq!(deps_id.as_str(), "deps_new");
    assert_eq!(index_snapshot.id.as_str(), "index_new");
    assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_new");

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn p8_apply_changes_ignores_set_deps_snapshot() {
    let mut host = AnalysisHostV2::default();

    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old.clone(),
        deps: deps_old,
    });

    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_old"), None);

    let deps_new = make_deps();
    let deps_id_new = DepsSnapshotId::from_hash("deps_new");
    runtime.apply_changes(vec![Change::SetDepsSnapshot {
        deps_id: deps_id_new,
        deps: deps_new,
    }]);

    let (analysis, _index_snapshot, deps_id) = runtime.snapshot_with_deps().await;
    assert_eq!(deps_id.as_str(), "deps_old");
    assert_eq!(analysis.deps_id().unwrap().as_str(), "deps_old");

    runtime.shutdown_for_test().await;
}

#[test]
fn ephemeral_snapshot_sets_contract_inputs() {
    let deps = make_deps();
    let deps_id = DepsSnapshotId::from_hash("deps_ephemeral");
    let settings = ExecutionSettings {
        settings_id: SettingsId::from_hash("settings_ephemeral"),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let snapshot = IntellisenseV2Facade::ephemeral_snapshot(
        deps_id.clone(),
        deps,
        make_index_snapshot("index_ephemeral"),
        settings.clone(),
        FileId(7),
        Arc::from("Перем х;"),
        42,
        Arc::from("<ephemeral>"),
    );

    assert_eq!(
        snapshot.analysis.file_version(FileId(7)).unwrap(),
        Some(42),
        "ephemeral snapshot should carry file version"
    );
    assert_eq!(
        snapshot.analysis.deps_id().unwrap().as_str(),
        deps_id.as_str(),
        "ephemeral snapshot should carry deps id"
    );
    assert_eq!(
        snapshot.analysis.settings_id().unwrap().as_str(),
        settings.settings_id.as_str(),
        "ephemeral snapshot should carry settings id"
    );
    assert_eq!(snapshot.index_snapshot.id.as_str(), "index_ephemeral");
}

#[test]
fn semantic_operation_contract_values_are_stable() {
    assert_eq!(SemanticOperation::Completion.as_str(), "completion");
    assert_eq!(SemanticOperation::Hover.as_str(), "hover");
    assert_eq!(SemanticOperation::SignatureHelp.as_str(), "signature_help");
    assert_eq!(SemanticOperation::Definition.as_str(), "definition");
    assert_eq!(
        SemanticOperation::DocumentSymbol.as_str(),
        "document_symbol"
    );
    assert_eq!(SemanticOperation::Rename.as_str(), "rename");
    assert_eq!(SemanticOperation::Diagnostics.as_str(), "diagnostics");
    assert_eq!(SemanticOperation::Members.as_str(), "members");
    assert_eq!(
        SemanticOperation::TypeAtPosition.as_str(),
        "type_at_position"
    );
    assert_eq!(SemanticOperation::SymbolSearch.as_str(), "symbol_search");
    assert_eq!(SemanticOperation::References.as_str(), "references");
}

#[test]
fn runtime_queue_priority_aligns_definition_with_interactive_operations() {
    for operation in [
        SemanticOperation::Completion,
        SemanticOperation::Hover,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
    ] {
        assert_eq!(
            RuntimeQueuePriority::for_operation(operation),
            RuntimeQueuePriority::Interactive,
            "{operation:?} must stay on interactive queue"
        );
    }

    assert_eq!(
        RuntimeQueuePriority::for_operation(SemanticOperation::DocumentSymbol),
        RuntimeQueuePriority::Background,
        "non-interactive operations must remain on background queue"
    );
}

#[test]
fn observability_contract_values_are_stable() {
    assert_eq!(
        ObservabilityStage::RuntimeQueueWait.as_str(),
        "runtime_queue_wait"
    );
    assert_eq!(
        ObservabilityStage::RuntimeWaitForFileVersion.as_str(),
        "runtime_wait_for_file_version"
    );
    assert_eq!(
        ObservabilityStage::RuntimeSnapshotWithDeps.as_str(),
        "runtime_snapshot_with_deps"
    );
    assert_eq!(ObservabilityStage::IrQuery.as_str(), "ir_query");
    assert_eq!(
        ObservabilityStage::SyntaxDiagnosticsQuery.as_str(),
        "syntax_diagnostics_query"
    );
    assert_eq!(
        ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
        "semantic_diagnostics_query"
    );
    assert_eq!(
        ObservabilityStage::ParseResultQuery.as_str(),
        "parse_result_query"
    );
    assert_eq!(SemanticOutcome::Success.as_str(), "success");
    assert_eq!(SemanticOutcome::Empty.as_str(), "empty");
    assert_eq!(SemanticOutcome::Cancelled.as_str(), "cancelled");
    assert_eq!(SemanticOutcome::Error.as_str(), "error");
    assert_eq!(SemanticOutcome::StaleVersion.as_str(), "stale_version");
    assert_eq!(SemanticOutcome::MissingDeps.as_str(), "missing_deps");
}

#[tokio::test]
async fn stateful_prepare_operation_returns_missing_deps_on_mismatch() {
    let mut host = AnalysisHostV2::default();
    let deps_old = make_deps();
    let deps_id_old = DepsSnapshotId::from_hash("deps_old");
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_old,
        deps: deps_old,
    });
    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index"), None);

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Hover,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: Some(DepsSnapshotId::from_hash("deps_expected")),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(matches!(result, Err(SemanticOutcome::MissingDeps)));

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_serves_stale_when_gap_within_default() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(10);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_ok");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_ok.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await
        .expect("interactive fallback should serve stale snapshot");
    assert!(
        prepared.wait_budget_exhausted,
        "expected bounded wait timeout for interactive path"
    );
    assert!(
        prepared.stale_served,
        "expected stale fallback to be served"
    );
    assert!(
        prepared.completion_churn_fastpath_active,
        "completion stale fallback under large churn should set churn-aware fastpath flag"
    );
    assert_eq!(prepared.observed_file_version, Some(4));
    assert!(
        prepared
            .wait_elapsed
            .is_some_and(|elapsed| elapsed >= Duration::from_millis(90)),
        "wait elapsed should reflect bounded wait timeout"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        counters.contains_key("intellisense_v2_interactive_wait_budget_exhausted_total"),
        "wait budget exhausted metric should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_interactive_stale_served_total"),
        "stale served metric should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_queue_wait_interactive_total"),
        "interactive queue-class counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_exec_interactive_total"),
        "interactive exec-class counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_completion_stale_fallback_total"),
        "completion stale-fallback counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_revision_lag_sample_total"),
        "revision lag counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_queue_wait_interactive_ms"),
        "interactive queue-class histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_exec_interactive_ms"),
        "interactive exec-class histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_revision_lag_versions"),
        "revision lag histogram should be recorded"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_prefers_latest_when_version_is_ready_under_large_churn() {
    let file_id = FileId(110);
    let deps_id = DepsSnapshotId::from_hash("deps_latest_ready");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 5,
        path: Arc::from("latest_ready.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, None)
        .await
        .expect("latest snapshot should be available without stale fallback");
    assert!(
        !prepared.wait_budget_exhausted,
        "latest path should not exceed wait budget when requested version is ready"
    );
    assert!(
        !prepared.stale_served,
        "stale fallback must not be served when latest version is already available"
    );
    assert_eq!(
        prepared.observed_file_version,
        Some(5),
        "prepared snapshot should observe requested latest file version"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn completion_mode_propagates_into_stage_drilldown_metrics() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(21);
    let deps_id = DepsSnapshotId::from_hash("deps_mode_split");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "mode_split",
        ))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 1,
        path: Arc::from("mode_split.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: Some("event_driven"),
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(1),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let prepared = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await
        .expect("prepare_stateful_operation");
    let analysis = prepared.snapshot.analysis;

    let _: Result<Option<()>, ()> = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok(None),
    );
    let _: Result<Option<()>, ()> = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::ParseResultQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok(None),
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_runtime_wait_for_file_version"
            ),
            "wait stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_runtime_snapshot_with_deps"
            ),
            "snapshot stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_ir_query"
            ),
            "ir stage counter must include completion mode dimension"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_parse_result_query"
            ),
            "parse_result stage counter must include completion mode dimension"
        );
    assert!(
        counters.contains_key("intellisense_v2_wait_for_file_version_completion_total"),
        "legacy wait counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_snapshot_completion_total"),
        "legacy snapshot counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_ir_query_completion_total"),
        "legacy ir counter must still be projected"
    );
    assert!(
        counters.contains_key("intellisense_v2_parse_result_query_total"),
        "legacy parse_result counter must still be projected"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_when_gap_exceeds_default() {
    let coordinator = SystemCoordinator::new();
    let file_id = FileId(11);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_reject");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 2,
        path: Arc::from("stale_reject.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime
        .prepare_stateful_operation(&context, Some(&coordinator))
        .await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "gap > 1 should reject stale fallback under default policy"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
    assert!(
            elapsed >= min_expected,
            "stale reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "stale reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        counters.contains_key("intellisense_v2_completion_fallback_unavailable_total"),
        "completion fallback-unavailable counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_revision_lag_sample_total"),
        "revision lag counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_revision_lag_versions"),
        "revision lag histogram should be recorded"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_when_age_exceeds_default() {
    let file_id = FileId(111);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_age");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_age.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    // Default max_stale_age is 1000ms; exceed it to verify age-based stale rejection.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime.prepare_stateful_operation(&context, None).await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "stale fallback must be rejected when stale age exceeds configured bound"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(400));
    assert!(
            elapsed >= min_expected,
            "stale-age reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "stale-age reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_completion_reports_missing_deps_before_stale_acceptance() {
    let file_id = FileId(112);
    let deps_id_actual = DepsSnapshotId::from_hash("deps_actual");
    let deps_id_requested = DepsSnapshotId::from_hash("deps_requested");
    let settings_id = SettingsId::from_hash("settings");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id_actual.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_deps_mismatch.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: true,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id_requested),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(
        matches!(result, Err(SemanticOutcome::MissingDeps)),
        "deps mismatch must short-circuit stale acceptance with MissingDeps"
    );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_on_settings_mismatch() {
    let file_id = FileId(12);
    let deps_id = DepsSnapshotId::from_hash("deps_stale_mismatch");
    let stale_settings_id = SettingsId::from_hash("settings_old");
    let requested_settings_id = SettingsId::from_hash("settings_new");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: make_deps(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: stale_settings_id,
        diagnostics_detail_level: DetailLevel::Full,
    });
    let runtime = IntellisenseV2Facade::new(
        host,
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    runtime.apply_changes(vec![Change::SetFile {
        file_id,
        text: Arc::from("x = 1;"),
        version: 4,
        path: Arc::from("stale_mismatch.bsl"),
    }]);
    let _ = runtime.snapshot().await;

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::SignatureHelp,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: Some(deps_id),
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: requested_settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let wait_budget_ms = crate::system::global_runtime_config()
        .get_u64(crate::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let result = runtime.prepare_stateful_operation(&context, None).await;
    let elapsed = started.elapsed();
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "settings mismatch must reject stale fallback"
    );
    let min_expected = Duration::from_millis(wait_budget_ms.saturating_sub(30));
    let max_expected = Duration::from_millis(wait_budget_ms.saturating_add(300));
    assert!(
            elapsed >= min_expected,
            "settings-mismatch reject should spend wait budget before fail (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );
    assert!(
            elapsed <= max_expected,
            "settings-mismatch reject should stay bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
        );

    runtime.shutdown_for_test().await;
}

#[tokio::test]
async fn interactive_prepare_timeout_rejects_stale_without_expected_deps() {
    let runtime = IntellisenseV2Facade::new(
        AnalysisHostV2::default(),
        Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash("p9"))),
        None,
    );
    let file_id = FileId(13);
    let settings_id = SettingsId::from_hash("settings");

    runtime.apply_changes(vec![
        Change::SetSettingsSnapshot {
            settings_id: settings_id.clone(),
            diagnostics_detail_level: DetailLevel::Full,
        },
        Change::SetFile {
            file_id,
            text: Arc::from("x = 1;"),
            version: 4,
            path: Arc::from("stale_no_expected_deps.bsl"),
        },
    ]);

    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id,
        min_file_version: Some(5),
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id,
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = runtime.prepare_stateful_operation(&context, None).await;
    assert!(
        matches!(result, Err(SemanticOutcome::StaleVersion)),
        "stale fallback must be rejected when expected deps snapshot is unknown"
    );

    runtime.shutdown_for_test().await;
}

#[test]
fn run_parse_result_query_skips_when_policy_disallows_it() {
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Hover,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let mut called = false;
    let result = IntellisenseV2Facade::run_parse_result_query(
        &context,
        &analysis,
        false,
        None,
        |_analysis| {
            called = true;
            Ok::<Option<()>, ()>(None)
        },
    )
    .expect("query should not fail");

    assert!(result.is_none(), "parse_result should be skipped by policy");
    assert!(
        !called,
        "query closure must not be called when policy skips"
    );
}

#[test]
fn run_optional_query_records_ir_metrics() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Completion,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Ok::<Option<()>, ()>(None),
    )
    .expect("query should succeed");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    assert!(
        counters.contains_key("intellisense_v2_ir_query_completion_total"),
        "IR counter should be recorded for completion"
    );
    assert!(
        histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
        "IR histogram should be recorded for completion"
    );
}

#[test]
fn run_optional_query_best_effort_downgrades_cancellation_to_empty() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let result = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    )
    .expect("best effort should downgrade cancellation");
    assert!(
        result.is_none(),
        "best effort cancellation must return empty"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let cancelled = counters
        .get("intellisense_v2_ir_query_cancelled_total_other")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        cancelled > 0,
        "best effort should still expose cancelled counters"
    );
}

#[test]
fn run_optional_query_ignore_drops_cancellation_counters() {
    let coordinator = SystemCoordinator::new();
    let analysis = AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Lsp,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("settings"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::Ignore,
    };

    let result = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    )
    .expect("ignore policy should drop cancellation error");
    assert!(result.is_none(), "ignore policy must return empty result");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let cancelled = counters
        .get("intellisense_v2_ir_query_cancelled_total_other")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert_eq!(
        cancelled, 0,
        "ignore policy should suppress cancelled counters"
    );
}

#[test]
fn singleflight_scope_is_bound_only_for_ir() {
    assert!(
        IntellisenseV2Facade::singleflight_requires_snapshot_identity(SingleflightQueryKind::Ir),
        "IR should remain tied to deps/settings snapshots"
    );
    assert!(
        !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
            SingleflightQueryKind::ParseResult
        ),
        "parse_result should not be tied to deps/settings snapshots"
    );
    assert!(
        !IntellisenseV2Facade::singleflight_requires_snapshot_identity(
            SingleflightQueryKind::SyntaxDiagnostics
        ),
        "syntax_diagnostics should not be tied to deps/settings snapshots"
    );
}

#[test]
fn singleflight_runs_leader_once_and_shares_result() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(777),
        file_version: 10,
        file_signature: "path:test://singleflight/777.bsl".to_string(),
        deps_id: Some(DepsSnapshotId::from_hash("deps")),
        settings_id: Some(SettingsId::from_hash("settings")),
        query_kind: SingleflightQueryKind::Ir,
    };
    let calls = Arc::new(AtomicUsize::new(0));

    let first_calls = calls.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::Ir,
            None,
            || {
                first_calls.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(60));
                Ok(Some(Arc::new(String::from("shared"))))
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_calls = calls.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::Ir,
            None,
            || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(String::from("second"))))
            },
        )
    });

    let first_result = first.join().expect("first thread join").expect("first ok");
    let second_result = second
        .join()
        .expect("second thread join")
        .expect("second ok");

    assert_eq!(
        first_result.as_ref().map(|value| value.as_str()),
        Some("shared")
    );
    assert_eq!(
        second_result.as_ref().map(|value| value.as_str()),
        Some("shared")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn singleflight_propagates_leader_cancel_without_retry_and_cleans_up() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(778),
        file_version: 10,
        file_signature: "path:test://singleflight/778.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::ParseResult,
    };
    let calls = Arc::new(AtomicUsize::new(0));

    let first_calls = calls.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::ParseResult,
            None,
            || {
                first_calls.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(60));
                Err(SingleflightQueryError::Cancelled)
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_calls = calls.clone();
    let second_key = key.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            second_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::ParseResult,
            None,
            || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(String::from("unexpected-retry"))))
            },
        )
    });

    let first_result = first.join().expect("first thread join");
    let second_result = second.join().expect("second thread join");
    assert!(first_result.is_err(), "leader must fail");
    assert!(
        second_result.is_err(),
        "follower must receive leader cancel"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "follower must not trigger retry inside the same flight"
    );

    let map = TEST_FLIGHTS
        .get()
        .expect("test singleflight map should be initialized");
    let inflight_len = map
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len();
    assert_eq!(inflight_len, 0, "flight entry must be cleaned up");

    let rerun_calls = calls.clone();
    let rerun = IntellisenseV2Facade::run_singleflight_query(
        &TEST_FLIGHTS,
        key,
        ObservabilityOrigin::Runtime,
        SingleflightQueryKind::ParseResult,
        None,
        || {
            rerun_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some(Arc::new(String::from("after-cleanup"))))
        },
    )
    .expect("new request after cleanup should run as new leader");
    assert_eq!(rerun.as_deref().map(String::as_str), Some("after-cleanup"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn singleflight_leader_panic_is_downgraded_and_cleans_up() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(780),
        file_version: 10,
        file_signature: "path:test://singleflight/780.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::SyntaxDiagnostics,
    };

    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            None,
            || {
                std::thread::sleep(std::time::Duration::from_millis(60));
                panic!("leader panic must not leak in-flight entry")
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            None,
            || Ok(Some(Arc::new(String::from("unexpected-after-panic")))),
        )
    });

    let first_result = first.join().expect("first thread join");
    let second_result = second.join().expect("second thread join");
    assert!(
        matches!(first_result, Err(SingleflightQueryError::Cancelled)),
        "leader panic must be exposed as cancelled outcome"
    );
    assert!(
        matches!(second_result, Err(SingleflightQueryError::Cancelled)),
        "follower must receive terminal leader outcome when panic happens"
    );

    let map = TEST_FLIGHTS
        .get()
        .expect("test singleflight map should be initialized");
    let inflight_len = map
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len();
    assert_eq!(
        inflight_len, 0,
        "singleflight key must be cleaned up after panic"
    );
}

#[test]
fn singleflight_records_leader_shared_and_wait_metrics() {
    static TEST_FLIGHTS: OnceLock<SingleflightMap<Arc<String>>> = OnceLock::new();
    let key = SingleflightRevisionKey {
        file_id: FileId(779),
        file_version: 10,
        file_signature: "path:test://singleflight/779.bsl".to_string(),
        deps_id: None,
        settings_id: None,
        query_kind: SingleflightQueryKind::SyntaxDiagnostics,
    };
    let coordinator = Arc::new(SystemCoordinator::new());

    let first_coordinator = coordinator.clone();
    let first_key = key.clone();
    let first = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            first_key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            Some(first_coordinator.as_ref()),
            || {
                std::thread::sleep(std::time::Duration::from_millis(50));
                Ok(Some(Arc::new(String::from("shared"))))
            },
        )
    });

    std::thread::sleep(std::time::Duration::from_millis(5));

    let second_coordinator = coordinator.clone();
    let second = std::thread::spawn(move || {
        IntellisenseV2Facade::run_singleflight_query(
            &TEST_FLIGHTS,
            key,
            ObservabilityOrigin::Runtime,
            SingleflightQueryKind::SyntaxDiagnostics,
            Some(second_coordinator.as_ref()),
            || Ok(Some(Arc::new(String::from("second")))),
        )
    });

    let _ = first.join().expect("first thread join").expect("first ok");
    let _ = second
        .join()
        .expect("second thread join")
        .expect("second ok");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    assert!(
        counters.contains_key("intellisense_v2_singleflight_leader_total"),
        "singleflight leader counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_singleflight_shared_total"),
        "singleflight shared counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_singleflight_wait_ms"),
        "singleflight wait histogram should be recorded"
    );
}

#[tokio::test]
async fn parity_stateful_and_ephemeral_diagnostics_are_equal() {
    let deps = make_deps();
    let deps_id = DepsSnapshotId::from_hash("deps_parity");
    let settings_id = SettingsId::from_hash("settings_parity");
    let settings = ExecutionSettings {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    };
    let file_id = FileId(11);
    let code: Arc<str> = Arc::from("Процедура Тест()\n\tМассив1.Добавить(1);\nКонецПроцедуры\n");
    let path: Arc<str> = Arc::from("parity_test.bsl");

    let mut host = AnalysisHostV2::default();
    host.apply_change(Change::SetDepsSnapshot {
        deps_id: deps_id.clone(),
        deps: deps.clone(),
    });
    host.apply_change(Change::SetSettingsSnapshot {
        settings_id: settings_id.clone(),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(Change::SetFile {
        file_id,
        text: code.clone(),
        version: 1,
        path: path.clone(),
    });
    let runtime = IntellisenseV2Facade::new(host, make_index_snapshot("index_parity"), None);
    let stateful = runtime.snapshot().await;

    let ephemeral = IntellisenseV2Facade::ephemeral_snapshot(
        deps_id,
        deps,
        make_index_snapshot("index_parity"),
        settings,
        file_id,
        code,
        1,
        path,
    )
    .analysis;

    let stateful_syntax = stateful
        .syntax_diagnostics(file_id)
        .expect("stateful syntax query")
        .unwrap_or_else(|| Arc::new(Vec::new()));
    let ephemeral_syntax = ephemeral
        .syntax_diagnostics(file_id)
        .expect("ephemeral syntax query")
        .unwrap_or_else(|| Arc::new(Vec::new()));

    let stateful_semantic = stateful
        .semantic_diagnostics(file_id)
        .expect("stateful semantic query")
        .unwrap_or_else(|| Arc::new(Vec::new()));
    let ephemeral_semantic = ephemeral
        .semantic_diagnostics(file_id)
        .expect("ephemeral semantic query")
        .unwrap_or_else(|| Arc::new(Vec::new()));

    let syntax_key =
        |d: &bsl_shared::domain::types::ParseError| (d.message.clone(), d.span.start, d.span.end);
    let semantic_key = |d: &bsl_shared::domain::types::TypeDiagnostic| {
        (
            d.message.clone(),
            d.span.start,
            d.span.end,
            format!("{:?}", d.severity),
        )
    };

    let mut left_syntax: Vec<_> = stateful_syntax.iter().map(syntax_key).collect();
    let mut right_syntax: Vec<_> = ephemeral_syntax.iter().map(syntax_key).collect();
    left_syntax.sort();
    right_syntax.sort();
    assert_eq!(
        left_syntax, right_syntax,
        "syntax diagnostics parity mismatch"
    );

    let mut left_semantic: Vec<_> = stateful_semantic.iter().map(semantic_key).collect();
    let mut right_semantic: Vec<_> = ephemeral_semantic.iter().map(semantic_key).collect();
    left_semantic.sort();
    right_semantic.sort();
    assert_eq!(
        left_semantic, right_semantic,
        "semantic diagnostics parity mismatch"
    );

    runtime.shutdown_for_test().await;
}
