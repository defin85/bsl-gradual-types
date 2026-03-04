use super::*;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio::time::timeout;

#[test]
fn parse_result_policy_keeps_diagnostics_enabled() {
    assert!(
        should_query_parse_result(SemanticOperation::Diagnostics, false),
        "diagnostics must keep parse_result query enabled for singleflight sharing"
    );
    assert!(
        !should_query_parse_result(SemanticOperation::Completion, false),
        "completion parse_result remains gated by IR availability"
    );
}

#[test]
fn diagnostics_profiles_follow_tiered_trigger_contract() {
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidChange),
        &[
            DiagnosticsProfile::Fast,
            DiagnosticsProfile::DebouncedFull,
            DiagnosticsProfile::IdleHeavy,
        ]
    );
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidOpen),
        &[DiagnosticsProfile::DebouncedFull]
    );
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::DidSave),
        &[DiagnosticsProfile::IdleHeavy]
    );
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::Idle),
        &[DiagnosticsProfile::IdleHeavy]
    );
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::DocumentsSet),
        &[DiagnosticsProfile::DebouncedFull]
    );
    assert_eq!(
        diagnostics_profiles_for_trigger(DiagnosticsTrigger::JobStart),
        &[DiagnosticsProfile::DebouncedFull]
    );
}

#[test]
fn diagnostics_execution_plan_matches_profile_expectations() {
    let fast = diagnostics_execution_plan(DiagnosticsProfile::Fast, true);
    assert!(!fast.run_syntax);
    assert!(!fast.run_semantic);
    assert!(!fast.flow_sensitive_semantic);
    assert_eq!(fast.cpu_class, CpuWorkClass::Interactive);

    let debounced = diagnostics_execution_plan(DiagnosticsProfile::DebouncedFull, true);
    assert!(debounced.run_syntax);
    assert!(debounced.run_semantic);
    assert!(!debounced.flow_sensitive_semantic);
    assert_eq!(debounced.cpu_class, CpuWorkClass::Background);

    let idle_heavy_flow_off = diagnostics_execution_plan(DiagnosticsProfile::IdleHeavy, false);
    assert!(idle_heavy_flow_off.run_syntax);
    assert!(idle_heavy_flow_off.run_semantic);
    assert!(!idle_heavy_flow_off.flow_sensitive_semantic);
    assert_eq!(idle_heavy_flow_off.cpu_class, CpuWorkClass::Background);

    let idle_heavy_flow_on = diagnostics_execution_plan(DiagnosticsProfile::IdleHeavy, true);
    assert!(idle_heavy_flow_on.flow_sensitive_semantic);
    assert_eq!(idle_heavy_flow_on.cpu_class, CpuWorkClass::Background);
}

#[test]
fn cpu_work_class_keeps_interactive_tools_out_of_background_queue() {
    assert_eq!(
        cpu_work_class_for_operation(SemanticOperation::TypeAtPosition),
        CpuWorkClass::Interactive
    );
    assert_eq!(
        cpu_work_class_for_operation(SemanticOperation::Members),
        CpuWorkClass::Interactive
    );
    assert_eq!(
        cpu_work_class_for_operation(SemanticOperation::Definition),
        CpuWorkClass::Interactive
    );
    assert_eq!(
        cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
        CpuWorkClass::Background
    );
    assert_eq!(
        cpu_work_class_for_operation(SemanticOperation::References),
        CpuWorkClass::Background
    );
}

struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }

    fn unset(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            std::env::set_var(self.key, prev);
        } else {
            std::env::remove_var(self.key);
        }
        global_runtime_config().reload_env_bootstrap_from_env();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cpu_budget_allows_borrow_when_other_queue_idle() {
    let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
    let _first = budget.acquire(CpuWorkClass::Interactive).await;

    let budget_clone = budget.clone();
    let borrowed = timeout(Duration::from_millis(150), async move {
        let _permit = budget_clone.acquire(CpuWorkClass::Interactive).await;
    })
    .await;
    assert!(
        borrowed.is_ok(),
        "second interactive acquire should borrow background permit when background queue is idle"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cpu_budget_background_progresses_under_interactive_load() {
    let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
    let interactive_reserved = budget.acquire(CpuWorkClass::Interactive).await;

    let (borrowed_ready_tx, borrowed_ready_rx) = oneshot::channel::<()>();
    let (borrowed_release_tx, borrowed_release_rx) = oneshot::channel::<()>();

    let budget_for_borrowed = budget.clone();
    let borrowed_task = tokio::spawn(async move {
        let permit = budget_for_borrowed.acquire(CpuWorkClass::Interactive).await;
        let _ = borrowed_ready_tx.send(());
        let _ = borrowed_release_rx.await;
        drop(permit);
    });
    borrowed_ready_rx
        .await
        .expect("borrowed interactive task should signal readiness");

    let budget_for_background = budget.clone();
    let background_task = tokio::spawn(async move {
        budget_for_background
            .acquire(CpuWorkClass::Background)
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !background_task.is_finished(),
        "background acquire must wait while both permits are occupied by interactive load"
    );

    drop(interactive_reserved);
    let background_permit = timeout(Duration::from_millis(250), background_task)
        .await
        .expect("background should make progress after one interactive permit is released")
        .expect("background task join should succeed");
    drop(background_permit);

    let _ = borrowed_release_tx.send(());
    borrowed_task.await.expect("borrowed interactive task join");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cpu_budget_interactive_progresses_under_background_load() {
    let budget = Arc::new(CpuBoundBudget::with_total_permits(2));
    let background_reserved = budget.acquire(CpuWorkClass::Background).await;

    let (borrowed_ready_tx, borrowed_ready_rx) = oneshot::channel::<()>();
    let (borrowed_release_tx, borrowed_release_rx) = oneshot::channel::<()>();

    let budget_for_borrowed = budget.clone();
    let borrowed_task = tokio::spawn(async move {
        let permit = budget_for_borrowed.acquire(CpuWorkClass::Background).await;
        let _ = borrowed_ready_tx.send(());
        let _ = borrowed_release_rx.await;
        drop(permit);
    });
    borrowed_ready_rx
        .await
        .expect("borrowed background task should signal readiness");

    let budget_for_interactive = budget.clone();
    let interactive_task = tokio::spawn(async move {
        budget_for_interactive
            .acquire(CpuWorkClass::Interactive)
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !interactive_task.is_finished(),
        "interactive acquire must wait while both permits are occupied by background load"
    );

    drop(background_reserved);
    let interactive_permit = timeout(Duration::from_millis(250), interactive_task)
        .await
        .expect("interactive should make progress after one background permit is released")
        .expect("interactive task join should succeed");
    drop(interactive_permit);

    let _ = borrowed_release_tx.send(());
    borrowed_task.await.expect("borrowed background task join");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cpu_budget_background_does_not_take_shared_while_interactive_waits() {
    let budget = Arc::new(CpuBoundBudget::with_total_permits(3));
    let interactive_reserved = budget.acquire(CpuWorkClass::Interactive).await;
    let background_reserved = budget.acquire(CpuWorkClass::Background).await;
    let shared_taken_by_background = budget.acquire(CpuWorkClass::Background).await;

    let budget_for_interactive = budget.clone();
    let interactive_waiter = tokio::spawn(async move {
        budget_for_interactive
            .acquire(CpuWorkClass::Interactive)
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !interactive_waiter.is_finished(),
        "interactive waiter should be queued while all permits are occupied"
    );

    let budget_for_background = budget.clone();
    let background_waiter = tokio::spawn(async move {
        budget_for_background
            .acquire(CpuWorkClass::Background)
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !background_waiter.is_finished(),
        "background waiter should also be queued before shared release"
    );

    drop(shared_taken_by_background);
    let interactive_permit = timeout(Duration::from_millis(300), interactive_waiter)
        .await
        .expect("interactive waiter should win released shared permit")
        .expect("interactive waiter join should succeed");

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !background_waiter.is_finished(),
        "background waiter must not steal shared while interactive queue is non-empty"
    );

    drop(interactive_permit);
    drop(interactive_reserved);
    drop(background_reserved);
    let background_permit = timeout(Duration::from_millis(300), background_waiter)
        .await
        .expect("background waiter should eventually make progress")
        .expect("background waiter join should succeed");
    drop(background_permit);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cpu_budget_bidirectional_waiters_eventually_make_progress() {
    let budget = Arc::new(CpuBoundBudget::with_total_permits(3));
    let interactive_reserved = budget.acquire(CpuWorkClass::Interactive).await;
    let background_reserved = budget.acquire(CpuWorkClass::Background).await;
    let shared_taken_by_interactive = budget.acquire(CpuWorkClass::Interactive).await;

    let budget_for_background = budget.clone();
    let mut background_waiter = tokio::spawn(async move {
        budget_for_background
            .acquire(CpuWorkClass::Background)
            .await
    });
    let budget_for_interactive = budget.clone();
    let mut interactive_waiter = tokio::spawn(async move {
        budget_for_interactive
            .acquire(CpuWorkClass::Interactive)
            .await
    });

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        !background_waiter.is_finished(),
        "background waiter should queue while permits are saturated"
    );
    assert!(
        !interactive_waiter.is_finished(),
        "interactive waiter should queue while permits are saturated"
    );

    drop(shared_taken_by_interactive);
    let (first_finished, first_result) = timeout(Duration::from_millis(300), async {
        tokio::select! {
            res = &mut interactive_waiter => ("interactive", res),
            res = &mut background_waiter => ("background", res),
        }
    })
    .await
    .expect("at least one waiter should make progress after shared release");
    let first_permit = first_result.expect("first waiter join should succeed");
    drop(first_permit);

    drop(interactive_reserved);
    drop(background_reserved);
    match first_finished {
        "interactive" => {
            let background_permit = timeout(Duration::from_millis(300), background_waiter)
                .await
                .expect("background waiter should eventually make progress")
                .expect("background waiter join should succeed");
            drop(background_permit);
        }
        "background" => {
            let interactive_permit = timeout(Duration::from_millis(300), interactive_waiter)
                .await
                .expect("interactive waiter should eventually make progress")
                .expect("interactive waiter join should succeed");
            drop(interactive_permit);
        }
        _ => unreachable!("unexpected waiter class"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_spawn_records_runtime_class_metrics() {
    let coordinator = SystemCoordinator::new();

    let interactive = spawn_bounded_blocking_with_class_observed(
        CpuWorkClass::Interactive,
        Some(&coordinator),
        || 1_u32,
    )
    .await
    .expect("interactive spawn should succeed");
    assert_eq!(interactive, 1);

    let background = spawn_bounded_blocking_with_class_observed(
        CpuWorkClass::Background,
        Some(&coordinator),
        || 2_u32,
    )
    .await
    .expect("background spawn should succeed");
    assert_eq!(background, 2);

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    assert!(
        counters.contains_key("intellisense_v2_runtime_queue_wait_interactive_total"),
        "interactive queue counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_queue_wait_background_total"),
        "background queue counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_exec_interactive_total"),
        "interactive exec counter should be recorded"
    );
    assert!(
        counters.contains_key("intellisense_v2_runtime_exec_background_total"),
        "background exec counter should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_queue_wait_interactive_ms"),
        "interactive queue histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_queue_wait_background_ms"),
        "background queue histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_exec_interactive_ms"),
        "interactive exec histogram should be recorded"
    );
    assert!(
        histograms.contains_key("intellisense_v2_runtime_exec_background_ms"),
        "background exec histogram should be recorded"
    );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_saturation_sample_total_origin_runtime_reason_queue_wait_work_class_interactive"
            ),
            "interactive drilldown queue_wait counter should be recorded"
        );
    assert!(
            counters.contains_key(
                "intellisense_v2_drilldown_saturation_sample_total_origin_runtime_reason_queue_wait_work_class_background"
            ),
            "background drilldown queue_wait counter should be recorded"
        );
    assert!(
            histograms.contains_key(
                "intellisense_v2_drilldown_saturation_sample_latency_ms_origin_runtime_reason_exec_work_class_interactive"
            ),
            "interactive drilldown exec histogram should be recorded"
        );
    assert!(
            histograms.contains_key(
                "intellisense_v2_drilldown_saturation_sample_latency_ms_origin_runtime_reason_exec_work_class_background"
            ),
            "background drilldown exec histogram should be recorded"
        );
    assert!(
        gauges.contains_key("intellisense_v2_runtime_saturation_waiters_interactive"),
        "legacy saturation gauge should be exported"
    );
    assert!(
            gauges.contains_key(
                "intellisense_v2_drilldown_saturation_gauge_origin_runtime_saturation_metric_queue_depth_total"
            ),
            "drilldown saturation gauge should be exported"
        );
}

#[test]
fn cpu_budget_reserves_extra_interactive_permit_when_capacity_allows() {
    let budget = CpuBoundBudget::with_total_permits(4);
    let snapshot = budget.saturation_snapshot();
    assert_eq!(
        snapshot.interactive_permits, 2,
        "interactive pool should get extra reserved permit on wider runtimes"
    );
    assert_eq!(
        snapshot.background_permits, 1,
        "background pool should keep one dedicated permit"
    );
    assert_eq!(
        snapshot.shared_permits, 1,
        "remaining capacity should stay shared"
    );
}

#[test]
fn interactive_knobs_clamp_and_emit_metric() {
    let _env_guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");

    let _wait_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS", "999999");
    let _gap_guard = EnvVarGuard::set(
        "BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_VERSION_GAP",
        "999",
    );
    let _age_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_AGE_MS", "999999");

    let coordinator = SystemCoordinator::new();
    let knobs = interactive_freshness_knobs(SemanticOperation::Completion, Some(&coordinator))
        .expect("completion should use interactive knobs");
    assert_eq!(knobs.wait_budget, Duration::from_millis(2000));
    assert_eq!(knobs.max_stale_version_gap, 10);
    assert_eq!(knobs.max_stale_age, Duration::from_millis(10_000));

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        counters.contains_key("intellisense_v2_interactive_knob_clamped_total"),
        "clamped interactive knobs should emit metric"
    );
}

#[test]
fn interactive_knobs_cover_all_interactive_operations() {
    let coordinator = SystemCoordinator::new();

    for operation in [
        SemanticOperation::Completion,
        SemanticOperation::Hover,
        SemanticOperation::SignatureHelp,
        SemanticOperation::Definition,
    ] {
        assert!(
            interactive_freshness_knobs(operation, Some(&coordinator)).is_some(),
            "{operation:?} must use interactive freshness knobs"
        );
    }

    assert!(
        interactive_freshness_knobs(SemanticOperation::DocumentSymbol, Some(&coordinator))
            .is_none(),
        "background operations must not use interactive freshness knobs"
    );
}

#[test]
fn completion_pipeline_knobs_use_defaults_when_env_missing() {
    let _env_guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");

    let _mode_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_MODE");
    let _canary_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT");
    let _capacity_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY");

    let knobs = CompletionPipelineKnobs::from_runtime_config();
    assert_eq!(knobs.mode, CompletionMode::On);
    assert_eq!(knobs.canary_percent, 0);
    assert_eq!(knobs.queue_capacity, 256);
}

#[test]
fn completion_pipeline_knobs_normalize_mode_and_clamp_values() {
    let _env_guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");

    let _mode_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_MODE", "CANARY");
    let _canary_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT", "999");
    let _capacity_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY", "1");

    let knobs = CompletionPipelineKnobs::from_runtime_config();
    assert_eq!(knobs.mode, CompletionMode::Canary);
    assert_eq!(knobs.canary_percent, 100);
    assert_eq!(knobs.queue_capacity, 16);
}

#[test]
fn completion_pipeline_knobs_fallback_to_off_for_unknown_mode() {
    let _env_guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");

    let _mode_guard = EnvVarGuard::set("BSL_INTELLISENSE_V2_COMPLETION_MODE", "legacy_like");
    let knobs = CompletionPipelineKnobs::from_runtime_config();
    assert_eq!(knobs.mode, CompletionMode::Off);
}

#[test]
fn scale_aware_knobs_use_defaults_when_env_missing() {
    let _env_guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");

    let _enabled_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_SCALE_AWARE_POLICY_ENABLED");
    let _bytes_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_BYTES");
    let _lines_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_LINES");
    let _window_guard = EnvVarGuard::unset("BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_WINDOW_MS");
    let _min_changes_guard =
        EnvVarGuard::unset("BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_MIN_CHANGES");

    let knobs = ScaleAwareDiagnosticsKnobs::from_runtime_config();
    assert!(knobs.enabled);
    assert_eq!(knobs.large_doc_bytes, 64 * 1024);
    assert_eq!(knobs.large_doc_lines, 2_000);
    assert_eq!(knobs.churn_window, Duration::from_millis(1_500));
    assert_eq!(knobs.churn_min_changes, 6);
}

#[test]
fn scale_aware_document_classification_uses_bytes_or_lines_threshold() {
    let knobs = ScaleAwareDiagnosticsKnobs {
        enabled: true,
        large_doc_bytes: 10,
        large_doc_lines: 3,
        churn_window: Duration::from_millis(1_500),
        churn_min_changes: 6,
    };

    assert!(
        !scale_aware_document_is_large("abc", knobs),
        "tiny one-line document should stay small"
    );
    assert!(
        scale_aware_document_is_large("0123456789", knobs),
        "byte threshold should classify document as large"
    );
    assert!(
        scale_aware_document_is_large("a\nb\nc", knobs),
        "line threshold should classify document as large"
    );
}

#[test]
fn completion_missing_ir_policy_decision_is_deterministic() {
    assert_eq!(
        completion_missing_ir_policy_decision(true, true, true, true),
        CompletionMissingIrPolicyDecision::StrictCacheIncomplete
    );
    assert_eq!(
        completion_missing_ir_policy_decision(false, false, true, true),
        CompletionMissingIrPolicyDecision::EmptyForNonMemberAccess
    );
    assert_eq!(
        completion_missing_ir_policy_decision(false, true, true, true),
        CompletionMissingIrPolicyDecision::DegradedIncomplete
    );
    assert_eq!(
        completion_missing_ir_policy_decision(false, true, false, true),
        CompletionMissingIrPolicyDecision::RelaxedCacheIncomplete
    );
    assert_eq!(
        completion_missing_ir_policy_decision(false, true, false, false),
        CompletionMissingIrPolicyDecision::KeywordFallbackUnavailable
    );
}

#[test]
fn completion_fastpath_preconditions_require_completion_version_deps_and_knobs() {
    let ready = completion_fastpath_preconditions(
        SemanticOperation::Completion,
        true,
        Some(42),
        true,
        true,
    );
    assert!(ready.can_attempt_bounded_stale_fallback());
    assert!(ready.churn_aware_fastpath_active());

    let missing_deps = completion_fastpath_preconditions(
        SemanticOperation::Completion,
        true,
        Some(42),
        false,
        true,
    );
    assert!(!missing_deps.can_attempt_bounded_stale_fallback());
    assert!(!missing_deps.churn_aware_fastpath_active());

    let non_completion =
        completion_fastpath_preconditions(SemanticOperation::Hover, true, Some(42), true, true);
    assert!(!non_completion.can_attempt_bounded_stale_fallback());
    assert!(!non_completion.churn_aware_fastpath_active());
}

#[test]
fn completion_fastpath_preconditions_expose_large_churn_without_forcing_fallback() {
    let stable_mode = completion_fastpath_preconditions(
        SemanticOperation::Completion,
        false,
        Some(7),
        true,
        true,
    );
    assert!(
        stable_mode.can_attempt_bounded_stale_fallback(),
        "stale fallback contract stays available outside churn when other preconditions pass"
    );
    assert!(
        !stable_mode.churn_aware_fastpath_active(),
        "churn-specific fastpath flag should only activate under large churn"
    );
}
