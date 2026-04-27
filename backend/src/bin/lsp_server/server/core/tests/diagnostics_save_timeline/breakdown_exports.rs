#[tokio::test]
async fn diagnostics_save_timeline_classifies_followup_readiness_wait_bucket() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///diagnostics-save-readiness-bucket.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1224),
        diagnostics_generation: 41,
        save_cycle_sequence: 13,
        requested_version: 22,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_followup_wait_state(
        &uri,
        key,
        "semantic_work",
        None,
        None,
        Some(Duration::from_millis(12)),
        Some(Duration::from_millis(80)),
        Some("recomputed"),
        Some("generic_pipeline"),
        Some("salsa"),
        Some("salsa"),
        None,
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_readiness_blocker_bucket.as_deref(),
        Some("wait_for_file_version")
    );
    assert_eq!(trace.followup_unclassified_readiness_residual_ms, None);
}

#[tokio::test]
async fn diagnostics_save_timeline_classifies_followup_publish_readiness_bucket() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///diagnostics-save-publish-readiness-bucket.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1226),
        diagnostics_generation: 43,
        save_cycle_sequence: 15,
        requested_version: 24,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "idle_heavy".to_string(),
                publish_kind: "full".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 144,
                runtime_queue_wait_ms: None,
                apply_lag_ms: Some(62),
                wait_for_file_version_ms: Some(62),
                snapshot_with_deps_ms: Some(7),
                ..Default::default()
            }),
        },
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_readiness_blocker_bucket.as_deref(),
        Some("wait_for_file_version")
    );
    assert_eq!(trace.followup_unclassified_readiness_residual_ms, None);
}

#[tokio::test]
async fn diagnostics_save_timeline_classifies_program_lowering_tail_before_snapshot_with_deps() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///diagnostics-save-program-lowering-tail-bucket.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1227),
        diagnostics_generation: 44,
        save_cycle_sequence: 16,
        requested_version: 25,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_followup_wait_state(
        &uri,
        key,
        "semantic_work",
        None,
        None,
        None,
        Some(Duration::from_millis(47)),
        Some("recomputed"),
        Some("detached_ready_artifacts"),
        Some("snapshot"),
        Some("snapshot_build"),
        None,
    );
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        None,
        Some("timeout"),
        None,
        Some(false),
        Some(diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2 {
            timeout_phase: Some("parse_exec"),
            timeout_leaf: Some("program_lowering"),
            parse_exec_ms: Some(3_598),
            parse_exec_core_build_exact_ready_snapshot_assembly_ms: Some(3_596),
            parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms: Some(
                3_596,
            ),
            parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms: Some(3_596),
            ready_install_ms: Some(1),
            ..Default::default()
        }),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_readiness_blocker_bucket.as_deref(),
        Some("program_lowering_tail")
    );
    assert_eq!(trace.followup_snapshot_with_deps_ms, Some(47));
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("detached_ready_artifacts")
    );
    assert_eq!(trace.followup_ready_snapshot_ready_install_ms, Some(1));
    assert_eq!(trace.followup_unclassified_readiness_residual_ms, None);
}

#[tokio::test]
async fn diagnostics_save_timeline_marks_unclassified_ready_install_residual() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///diagnostics-save-unclassified-residual.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1225),
        diagnostics_generation: 42,
        save_cycle_sequence: 14,
        requested_version: 23,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        None,
        None,
        None,
        Some(true),
        Some(diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2 {
            parse_exec_ms: Some(84),
            ready_install_ms: Some(2_193),
            ..Default::default()
        }),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_readiness_blocker_bucket.as_deref(),
        Some("unclassified_readiness_residual")
    );
    assert_eq!(
        trace.followup_unclassified_readiness_residual_ms,
        Some(2_193)
    );
}

#[tokio::test]
async fn p24_diagnostics_save_timeline_reports_relief_valve_timeout_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24-ready-snapshot-relief-timeout.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(144);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 34,
        save_cycle_sequence: 10,
        requested_version: 12,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 12;\nEndProcedure\n");
    let exact_text_hash = *blake3::hash(exact_text.as_bytes()).as_bytes();
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server
        .diagnostics_generation_v2
        .write()
        .await
        .insert(file_id, key.diagnostics_generation);
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, key.requested_version);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: key.requested_version,
            text: exact_text.clone(),
        },
    );
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    server
        .background_parse_snapshot_apply_tasks_v2
        .lock()
        .await
        .insert(
            file_id,
            crate::server::BackgroundParseSnapshotApplyTaskV2 {
                target_epoch: Arc::new(std::sync::atomic::AtomicU64::new(1)),
                target: Arc::new(std::sync::Mutex::new(
                    crate::server::BackgroundParseSnapshotApplyTargetV2 {
                        requested_version: key.requested_version,
                        text_hash: exact_text_hash,
                        save_cycle_sequence: None,
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::<str>::from(uri.path().to_string()),
                        text: exact_text,
                        parser_base_recovery_text: None,
                        parser_base_recovery_reuse_parse_result: None,
                        parser_edits: Vec::new(),
                        forced_full_parse_reason: None,
                        async_delay_mode: crate::server::ParseSnapshotAsyncDelayMode::None,
                        blocking_delay_env_key: None,
                        did_change_attribution: None,
                        epoch: 1,
                    },
                )),
                control,
                handle: tokio::spawn(async {}),
            },
        );

    let disposition = server
        .maybe_execute_save_followup_ready_snapshot_relief_valve_v2(
            &uri,
            &supersession_key,
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
            None,
            Instant::now(),
            false,
            false,
            None,
            None,
        )
        .await;
    assert!(
        disposition.is_none(),
        "relief valve timeout scenario must remain unpublished, disposition={disposition:?}"
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    let relief_budget_ms =
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
            .as_millis()
            .min(u64::MAX as u128) as u64;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_budget_ms,
        Some(relief_budget_ms)
    );
    assert!(
        trace
            .followup_ready_snapshot_relief_valve_elapsed_ms
            .is_some_and(|value| value >= relief_budget_ms.saturating_sub(50)),
        "relief timeout trace must expose spent extra wait close to budget, trace={trace:?}"
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("exhausted_continuation_proof")
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_timeout"
        )) > 0,
        "relief timeout path must export timeout probe counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_timed_out"
        )) > 0,
        "relief timeout path must export explicit timeout outcome counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_exhausted_continuation_proof"
        )) > 0,
        "relief timeout path must export exhausted continuation proof counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24b_diagnostics_save_timeline_exports_program_lowering_reuse_summary() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24b-program-lowering-reuse-summary.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(244),
        diagnostics_generation: 44,
        save_cycle_sequence: 12,
        requested_version: 14,
    };
    let completed = crate::server::ReadyParseSnapshotPhaseAttributionV2 {
        parse_exec_ms: Some(84),
        parse_exec_core_parse_build_ms: Some(84),
        parse_exec_core_build_pre_parse_setup_ms: None,
        parse_exec_core_build_parser_base_recovery_ms: None,
        parse_exec_core_build_parser_tree_build_ms: Some(8),
        parse_exec_core_build_exact_ready_snapshot_assembly_ms: Some(76),
        parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms: Some(76),
        parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms: Some(70),
        parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms: Some(
            6,
        ),
        parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms: None,
        parse_exec_core_build_tree_cache_install_ms: None,
        parse_exec_optional_cache_enrichment_ms: None,
        post_parse_pre_materialization_ms: Some(5),
        ready_install_ms: Some(3),
        document_symbol_side_work_ms: None,
    };
    let summary = bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringSummary {
        reuse_outcome: bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseOutcome::RoutineBodyReuse,
        reused_lowering_units: 41,
        rebuilt_lowering_units: 7,
        reused_window_count: 2,
        rebuilt_window_count: 1,
        largest_rebuilt_window_lowering_units: 7,
        fully_reused_top_level_node_count: 1,
        fully_rebuilt_top_level_node_count: 0,
        routine_body_reuse_node_count: 1,
        fully_reused_top_level_lowering_units: 30,
        fully_rebuilt_top_level_lowering_units: 0,
        routine_body_reused_prefix_lowering_units: 7,
        routine_body_reused_suffix_lowering_units: 4,
        routine_body_rebuilt_lowering_units: 7,
        reuse_plan_build_source: Some(
            bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReusePlanBuildSource::Owned,
        ),
        reuse_seed_source: Some(
            bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseSeedSource::AstCacheOwned,
        ),
        reuse_seed_candidate_count: Some(1),
        reuse_seed_eviction_reason: None,
        reuse_plan_failure_reason: None,
        reuse_plan_take_if_unique_hit: Some(true),
        reuse_plan_borrowed_cache_hit: Some(false),
        reuse_plan_build_ms: Some(9),
        reuse_plan_owned_build_ms: Some(9),
        reuse_plan_borrowed_build_ms: None,
        reuse_plan_rebase_ms: Some(6),
        reuse_plan_rebase_statement_count: Some(5),
        reused_progress_ms: Some(18),
        reused_progress_call_count: Some(2),
        rebuild_dispatch_ms: Some(31),
        rebuild_dispatch_call_count: Some(1),
        rebuild_dispatch_callable_ms: Some(31),
        rebuild_dispatch_callable_call_count: Some(1),
        rebuild_dispatch_callable_body_dispatch_ms: Some(11),
        rebuild_dispatch_callable_body_dispatch_call_count: Some(1),
        rebuild_dispatch_callable_non_body_dispatch_ms: Some(20),
        rebuild_dispatch_control_flow_ms: Some(0),
        rebuild_dispatch_control_flow_call_count: Some(0),
        rebuild_dispatch_simple_ms: Some(0),
        rebuild_dispatch_simple_call_count: Some(0),
        rebuild_dispatch_other_ms: Some(0),
        rebuild_dispatch_other_call_count: Some(0),
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_completed(
            &completed,
            Some(&summary),
        )
        .expect("completed phase attribution with lowering summary");
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some("ready"),
        Some("ready"),
        Some("ready_same_version"),
        Some(true),
        Some(attribution),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome
            .as_deref(),
        Some("routine_body_reuse")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units,
        Some(41)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units,
        Some(7)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count,
        Some(2)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units,
        Some(7)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units,
        Some(30)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units,
        Some(7)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units,
        Some(4)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units,
        Some(7)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source
            .as_deref(),
        Some("owned")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source
            .as_deref(),
        Some("ast_cache_owned")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason,
        None
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason,
        None
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit,
        Some(true)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit,
        Some(false)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms,
        Some(9)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms,
        Some(9)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms,
        None
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms,
        Some(6)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count,
        Some(5)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms,
        Some(18)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count,
        Some(2)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms,
        Some(31)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms,
        Some(31)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms,
        Some(11)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms,
        Some(20)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count,
        Some(0)
    );
}

#[tokio::test]
async fn p24b_diagnostics_save_timeline_snapshot_exports_program_lowering_reuse_summary() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p24b-snapshot-program-lowering-reuse-summary.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(246),
        diagnostics_generation: 46,
        save_cycle_sequence: 13,
        requested_version: 15,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());
    let summary = bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringSummary {
        reuse_outcome: bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseOutcome::TopLevelReuse,
        reused_lowering_units: 48,
        rebuilt_lowering_units: 0,
        reused_window_count: 1,
        rebuilt_window_count: 0,
        largest_rebuilt_window_lowering_units: 0,
        fully_reused_top_level_node_count: 3,
        fully_rebuilt_top_level_node_count: 0,
        routine_body_reuse_node_count: 0,
        fully_reused_top_level_lowering_units: 48,
        fully_rebuilt_top_level_lowering_units: 0,
        routine_body_reused_prefix_lowering_units: 0,
        routine_body_reused_suffix_lowering_units: 0,
        routine_body_rebuilt_lowering_units: 0,
        reuse_plan_build_source: Some(
            bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReusePlanBuildSource::Owned,
        ),
        reuse_seed_source: Some(
            bsl_runtime::system::parser_coordinator::ParseSnapshotProgramLoweringReuseSeedSource::AstCacheOwned,
        ),
        reuse_seed_candidate_count: Some(1),
        reuse_seed_eviction_reason: None,
        reuse_plan_failure_reason: None,
        reuse_plan_take_if_unique_hit: Some(true),
        reuse_plan_borrowed_cache_hit: Some(false),
        reuse_plan_build_ms: Some(4),
        reuse_plan_owned_build_ms: Some(4),
        reuse_plan_borrowed_build_ms: None,
        reuse_plan_rebase_ms: Some(0),
        reuse_plan_rebase_statement_count: Some(0),
        reused_progress_ms: Some(22),
        reused_progress_call_count: Some(3),
        rebuild_dispatch_ms: Some(0),
        rebuild_dispatch_call_count: Some(0),
        rebuild_dispatch_callable_ms: Some(0),
        rebuild_dispatch_callable_call_count: Some(0),
        rebuild_dispatch_callable_body_dispatch_ms: Some(0),
        rebuild_dispatch_callable_body_dispatch_call_count: Some(0),
        rebuild_dispatch_callable_non_body_dispatch_ms: Some(0),
        rebuild_dispatch_control_flow_ms: Some(0),
        rebuild_dispatch_control_flow_call_count: Some(0),
        rebuild_dispatch_simple_ms: Some(0),
        rebuild_dispatch_simple_call_count: Some(0),
        rebuild_dispatch_other_ms: Some(0),
        rebuild_dispatch_other_call_count: Some(0),
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly,
    );
    control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering,
    );
    control.set_program_lowering_summary(summary);
    tokio::time::sleep(Duration::from_millis(20)).await;

    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("snapshot phase attribution with lowering summary");
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some("not_ready"),
        Some("timeout"),
        Some("in_flight_same_version"),
        Some(true),
        Some(attribution),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome
            .as_deref(),
        Some("top_level_reuse")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units,
        Some(48)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units,
        Some(0)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source
            .as_deref(),
        Some("owned")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source
            .as_deref(),
        Some("ast_cache_owned")
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count,
        Some(1)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit,
        Some(true)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit,
        Some(false)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms,
        Some(22)
    );
    assert_eq!(
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms,
        Some(0)
    );
}

#[tokio::test]
async fn p24c_diagnostics_save_timeline_exports_semantic_diagnostics_query_breakdown() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24c-semantic-diagnostics-query-breakdown.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(245),
        diagnostics_generation: 45,
        save_cycle_sequence: 13,
        requested_version: 15,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 12,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(6),
                semantic_diagnostics_query_ms: None,
                semantic_diagnostics_inputs_ms: None,
                semantic_diagnostics_parse_result_ms: None,
                semantic_diagnostics_ir_ms: None,
                semantic_diagnostics_collect_ms: None,
                semantic_diagnostics_flow_sensitive_ms: None,
                semantic_diagnostics_ir_ast_to_ir_convert_ms: None,
                semantic_diagnostics_ir_semantic_facts_materialize_ms: None,
                semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_visit_statements_ms: None,
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: None,
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: None,
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_count: None,
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count: None,
                semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms:
                    None,
                semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms: None,
                semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count:
                    None,
                semantic_diagnostics_ir_semantic_facts_statement_count: None,
                semantic_diagnostics_ir_semantic_facts_local_function_summary_count: None,
                semantic_diagnostics_ir_semantic_facts_index_entry_count: None,
                publish_wait_ms: Some(1),
                ..Default::default()
            }),
        },
    );
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "idle_heavy".to_string(),
                publish_kind: "full".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 320,
                syntax_work_mode: Some("reused".to_string()),
                semantic_path: Some("ready_artifacts".to_string()),
                semantic_parse_source: Some("snapshot".to_string()),
                semantic_ir_source: Some("snapshot_build".to_string()),
                runtime_queue_wait_ms: Some(2),
                apply_lag_ms: None,
                blocking_queue_wait_ms: Some(4),
                wait_for_file_version_ms: Some(10),
                snapshot_with_deps_ms: Some(31),
                syntax_diagnostics_query_ms: None,
                semantic_diagnostics_query_ms: Some(29),
                semantic_diagnostics_inputs_ms: Some(3),
                semantic_diagnostics_parse_result_ms: Some(18),
                semantic_diagnostics_ir_ms: Some(5),
                semantic_diagnostics_collect_ms: Some(2),
                semantic_diagnostics_flow_sensitive_ms: Some(1),
                semantic_diagnostics_ir_ast_to_ir_convert_ms: Some(2),
                semantic_diagnostics_ir_semantic_facts_materialize_ms: Some(3),
                semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms:
                    Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms:
                    Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms:
                    Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count:
                    Some(2),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count:
                    Some(1),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count:
                    Some(3),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count:
                    Some(0),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count:
                    Some(1),
                semantic_diagnostics_ir_semantic_facts_visit_statements_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_count: Some(2),
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count: Some(3),
                semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms:
                    Some(4),
                semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count:
                    Some(2),
                semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms:
                    Some(5),
                semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count:
                    Some(3),
                semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms:
                    Some(6),
                semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count:
                    Some(4),
                semantic_diagnostics_ir_semantic_facts_statement_count: Some(8),
                semantic_diagnostics_ir_semantic_facts_local_function_summary_count: Some(1),
                semantic_diagnostics_ir_semantic_facts_index_entry_count: Some(5),
                publish_wait_ms: Some(2),
                ..Default::default()
            }),
        },
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    let followup_publish = trace
        .followup_publish
        .expect("idle_heavy publish must be retained as followup publish");
    assert_eq!(followup_publish.semantic_diagnostics_query_ms, Some(29));
    assert_eq!(followup_publish.semantic_diagnostics_inputs_ms, Some(3));
    assert_eq!(
        followup_publish.semantic_diagnostics_parse_result_ms,
        Some(18)
    );
    assert_eq!(followup_publish.semantic_diagnostics_ir_ms, Some(5));
    assert_eq!(followup_publish.semantic_diagnostics_collect_ms, Some(2));
    assert_eq!(
        followup_publish.semantic_diagnostics_flow_sensitive_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_ast_to_ir_convert_ms,
        Some(2)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_materialize_ms,
        Some(3)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_seed_module_context_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count,
        Some(2)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count,
        Some(3)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count,
        Some(0)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_statements_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_callable_body_count,
        Some(2)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count,
        Some(3)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms,
        Some(4)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count,
        Some(2)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms,
        Some(5)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count,
        Some(3)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms,
        Some(6)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count,
        Some(4)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_statement_count,
        Some(8)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summary_count,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_index_entry_count,
        Some(5)
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
}

#[tokio::test]
async fn p24d_diagnostics_save_timeline_exports_diagnostics_only_semantic_facts_breakdown() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24d-diagnostics-only-semantic-facts-query-breakdown.bsl")
        .expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(246),
        diagnostics_generation: 46,
        save_cycle_sequence: 14,
        requested_version: 16,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 12,
                syntax_work_mode: Some("recomputed".to_string()),
                syntax_diagnostics_query_ms: Some(6),
                publish_wait_ms: Some(1),
                ..Default::default()
            }),
        },
    );
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "idle_heavy".to_string(),
                publish_kind: "full".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 340,
                syntax_work_mode: Some("reused".to_string()),
                semantic_path: Some("ready_artifacts".to_string()),
                semantic_parse_source: Some("snapshot".to_string()),
                semantic_ir_source: Some("snapshot_build".to_string()),
                semantic_materialization_path: Some("diagnostics_only".to_string()),
                runtime_queue_wait_ms: Some(2),
                blocking_queue_wait_ms: Some(5),
                wait_for_file_version_ms: Some(9),
                snapshot_with_deps_ms: Some(33),
                semantic_diagnostics_query_ms: Some(47),
                semantic_diagnostics_inputs_ms: Some(4),
                semantic_diagnostics_parse_result_ms: Some(21),
                semantic_diagnostics_ir_ms: Some(29),
                semantic_diagnostics_collect_ms: Some(7),
                semantic_diagnostics_ir_ast_to_ir_convert_ms: Some(8),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms: Some(17),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms:
                    Some(2),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms:
                    Some(9),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms:
                    Some(1),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms:
                    Some(4),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms:
                    Some(2),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms:
                    Some(3),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count:
                    Some(4),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count:
                    Some(2),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count:
                    Some(5),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count:
                    Some(1),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count:
                    Some(1),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms:
                    Some(5),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms:
                    Some(4),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms:
                    Some(2),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_count:
                    Some(6),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_count:
                    Some(7),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_statement_count:
                    Some(13),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summary_count:
                    Some(4),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count:
                    Some(9),
                publish_wait_ms: Some(2),
                ..Default::default()
            }),
        },
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    let followup_publish = trace
        .followup_publish
        .expect("idle_heavy publish must be retained as followup publish");
    assert_eq!(
        followup_publish.semantic_materialization_path.as_deref(),
        Some("diagnostics_only")
    );
    assert_eq!(
        trace.followup_semantic_materialization_path.as_deref(),
        Some("diagnostics_only")
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms,
        Some(17)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms,
        Some(2)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms,
        Some(9)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms,
        Some(4)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms,
        Some(2)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms,
        Some(3)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count,
        Some(4)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count,
        Some(2)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count,
        Some(5)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count,
        Some(1)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms,
        Some(5)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms,
        Some(4)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms,
        Some(2)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_count,
        Some(6)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_count,
        Some(7)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_diagnostics_only_semantic_facts_statement_count,
        Some(13)
    );
    assert_eq!(
        followup_publish
            .semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summary_count,
        Some(4)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count,
        Some(9)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_materialize_ms,
        None
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_seed_module_context_ms,
        None
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms,
        None
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_statements_ms,
        None
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms,
        None
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms,
        None
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
}

#[tokio::test]
async fn p24e_diagnostics_save_timeline_exports_full_semantic_facts_fallback_materialization_path()
{
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p24e-full-semantic-facts-fallback-query-breakdown.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(247),
        diagnostics_generation: 47,
        save_cycle_sequence: 15,
        requested_version: 17,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 11,
                syntax_work_mode: Some("recomputed".to_string()),
                syntax_diagnostics_query_ms: Some(5),
                publish_wait_ms: Some(1),
                ..Default::default()
            }),
        },
    );
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "idle_heavy".to_string(),
                publish_kind: "full".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 360,
                syntax_work_mode: Some("reused".to_string()),
                semantic_path: Some("ready_artifacts".to_string()),
                semantic_parse_source: Some("snapshot".to_string()),
                semantic_ir_source: Some("snapshot_build".to_string()),
                semantic_materialization_path: Some("full_semantic_facts_fallback".to_string()),
                semantic_diagnostics_query_ms: Some(63),
                semantic_diagnostics_inputs_ms: Some(4),
                semantic_diagnostics_parse_result_ms: Some(22),
                semantic_diagnostics_ir_ms: Some(31),
                semantic_diagnostics_collect_ms: Some(6),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms: Some(12),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms: Some(
                    5,
                ),
                semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count: Some(8),
                semantic_diagnostics_ir_semantic_facts_materialize_ms: Some(19),
                semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: Some(3),
                semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: Some(7),
                semantic_diagnostics_ir_semantic_facts_visit_statements_ms: Some(4),
                semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: Some(2),
                semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: Some(1),
                semantic_diagnostics_ir_semantic_facts_statement_count: Some(10),
                semantic_diagnostics_ir_semantic_facts_local_function_summary_count: Some(2),
                semantic_diagnostics_ir_semantic_facts_index_entry_count: Some(8),
                publish_wait_ms: Some(2),
                ..Default::default()
            }),
        },
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    let followup_publish = trace
        .followup_publish
        .expect("idle_heavy publish must be retained as followup publish");
    assert_eq!(
        followup_publish.semantic_materialization_path.as_deref(),
        Some("full_semantic_facts_fallback")
    );
    assert_eq!(
        trace.followup_semantic_materialization_path.as_deref(),
        Some("full_semantic_facts_fallback")
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms,
        Some(12)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_materialize_ms,
        Some(19)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_seed_module_context_ms,
        Some(3)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms,
        Some(7)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_statements_ms,
        Some(4)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms,
        Some(2)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms,
        Some(1)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_statement_count,
        Some(10)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_local_function_summary_count,
        Some(2)
    );
    assert_eq!(
        followup_publish.semantic_diagnostics_ir_semantic_facts_index_entry_count,
        Some(8)
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
}
