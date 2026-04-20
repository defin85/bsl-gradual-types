#[tokio::test]
async fn p24_diagnostics_save_timeline_skips_relief_valve_for_non_exact_current_producer() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24-ready-snapshot-relief-skip-non-exact.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(145);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 35,
        save_cycle_sequence: 11,
        requested_version: 13,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let shadow_text: Arc<str> = Arc::from("Procedure Test()\n    Return 13;\nEndProcedure\n");
    let newer_text: Arc<str> = Arc::from("Procedure Test()\n    Return 14;\nEndProcedure\n");
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
        .insert(file_id, key.requested_version + 1);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: key.requested_version,
            text: shadow_text,
        },
    );
    server
        .background_parse_snapshot_apply_tasks_v2
        .lock()
        .await
        .insert(
            file_id,
            crate::server::BackgroundParseSnapshotApplyTaskV2 {
                target_epoch: Arc::new(std::sync::atomic::AtomicU64::new(2)),
                target: Arc::new(std::sync::Mutex::new(
                    crate::server::BackgroundParseSnapshotApplyTargetV2 {
                        requested_version: key.requested_version + 1,
                        text_hash: *blake3::hash(newer_text.as_bytes()).as_bytes(),
                        save_cycle_sequence: None,
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::<str>::from(uri.path().to_string()),
                        text: newer_text,
                        parser_base_recovery_text: None,
                        parser_base_recovery_reuse_parse_result: None,
                        parser_edits: Vec::new(),
                        forced_full_parse_reason: None,
                        async_delay_mode: crate::server::ParseSnapshotAsyncDelayMode::None,
                        blocking_delay_env_key: None,
                        did_change_attribution: None,
                        epoch: 2,
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
        "non-exact producer must skip relief valve and keep fallback path open"
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("skipped_not_exact_still_current")
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_budget_ms,
        Some(
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64
        )
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_elapsed_ms, None,
        "skip path must not spend extra wait budget, trace={trace:?}"
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_not_exact_still_current"
        )) > 0,
        "non-exact path must export explicit skip outcome counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p32_diagnostics_save_timeline_relief_valve_treats_late_did_save_task_as_exact_current() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p32-ready-snapshot-relief-late-did-save.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(1451);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 351,
        save_cycle_sequence: 111,
        requested_version: 131,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 131;\nEndProcedure\n");
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
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly,
    );
    control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering,
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
        "late didSave task should still spend bounded relief wait before fallback, disposition={disposition:?}"
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("exhausted_continuation_proof")
    );
    assert_ne!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("skipped_not_exact_still_current")
    );

    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_timed_out"
        )) > 0,
        "late didSave exact path must export engaged_timed_out instead of skip, counters={counters:?}"
    );
}

#[tokio::test]
async fn p32_diagnostics_save_timeline_continuation_reports_superseded_generation() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p32-ready-snapshot-continuation-superseded-generation.bsl")
        .expect("uri");
    let file_id = bsl_analysis_v2::FileId(1452);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 352,
        save_cycle_sequence: 112,
        requested_version: 132,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 132;\nEndProcedure\n");
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
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
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
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );
    let server_for_supersession = server.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(550)).await;
        server_for_supersession
            .diagnostics_generation_v2
            .write()
            .await
            .insert(file_id, key.diagnostics_generation + 1);
        control.control_notify.notify_waiters();
    });

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
    assert_eq!(
        disposition,
        Some(bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration)
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("superseded")
    );
    assert!(
        trace.followup_semantic_path.is_none(),
        "superseded continuation must not degrade into shadow_state semantic work, trace={trace:?}"
    );
    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_superseded"
        )) > 0,
        "superseded continuation must export continuation reason counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p32_diagnostics_save_timeline_continuation_reports_cancelled_after_bounded_core_parse_build_entry(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p32-ready-snapshot-continuation-cancelled-core-build.bsl")
        .expect("uri");
    let file_id = bsl_analysis_v2::FileId(1453);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 353,
        save_cycle_sequence: 113,
        requested_version: 133,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 133;\nEndProcedure\n");
    let exact_text_hash = *blake3::hash(exact_text.as_bytes()).as_bytes();
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());
    let cancel_token = crate::server::DiagnosticsCancellationTokenV2::new();

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
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
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
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );
    let cancel_token_for_cancel = cancel_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(550)).await;
        cancel_token_for_cancel
            .cancel(crate::server::DiagnosticsCancellationReasonV2::ClientCancel);
        control.control_notify.notify_waiters();
    });

    let disposition = server
        .maybe_execute_save_followup_ready_snapshot_relief_valve_v2(
            &uri,
            &supersession_key,
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
            Some(&cancel_token),
            Instant::now(),
            false,
            false,
            None,
            None,
        )
        .await;
    assert_eq!(
        disposition,
        Some(bsl_runtime::application::DiagnosticsDisposition::ClientCancel)
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("cancelled")
    );
    assert!(
        trace.followup_semantic_path.is_none(),
        "cancelled continuation must not degrade into shadow_state semantic work, trace={trace:?}"
    );
    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_cancelled"
        )) > 0,
        "cancelled continuation must export continuation reason counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24_diagnostics_save_timeline_reports_relief_valve_help_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24-ready-snapshot-relief-help.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(146);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 36,
        save_cycle_sequence: 12,
        requested_version: 14,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> =
        Arc::from("Procedure Test()\n    UndefinedValue = UnknownIdentifier;\nEndProcedure\n");
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
                        text: exact_text.clone(),
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
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );
    let server_for_ready = server.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let parse_snapshot = parse_snapshot_for_test(
            file_id,
            key.requested_version,
            exact_text.as_ref(),
            Vec::new(),
            false,
            None,
        );
        let ready_state = ReadyParseSnapshotStateV2 {
            text: exact_text,
            parse_snapshot,
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2 {
                parse_exec_ms: Some(3600),
                parse_exec_core_parse_build_ms: Some(3600),
                parse_exec_core_build_pre_parse_setup_ms: None,
                parse_exec_core_build_parser_base_recovery_ms: None,
                parse_exec_core_build_parser_tree_build_ms: Some(3600),
                parse_exec_core_build_exact_ready_snapshot_assembly_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                parse_exec_core_build_tree_cache_install_ms: None,
                parse_exec_optional_cache_enrichment_ms: None,
                post_parse_pre_materialization_ms: None,
                ready_install_ms: Some(1),
                document_symbol_side_work_ms: None,
            },
            program_lowering_summary: None,
        };
        server_for_ready
            .latest_ready_parse_snapshots_v2
            .write()
            .await
            .insert(file_id, ready_state);
        control.materialized_notify.notify_waiters();
    });

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
    assert_eq!(
        disposition,
        Some(bsl_runtime::application::DiagnosticsDisposition::Published)
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_helped")
    );
    assert!(
        trace.followup_ready_snapshot_continuation_reason.is_none(),
        "engaged_helped path must not set continuation reason, trace={trace:?}"
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_budget_ms,
        Some(
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64
        )
    );
    assert!(
        trace
            .followup_ready_snapshot_relief_valve_elapsed_ms
            .is_some_and(|value| value > 0),
        "engaged_helped path must expose spent relief wait, trace={trace:?}"
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_ready"
        )) > 0,
        "engaged_helped path must export relief-valve probe ready counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_helped"
        )) > 0,
        "engaged_helped path must export explicit valve outcome counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback() {
    let server = create_diagnostics_save_timeline_test_server();
    prime_server_with_syntax_helper_deps(&server).await;
    let uri = Url::parse("file:///p24b-detached-ready-artifacts-before-shadow-fallback.bsl")
        .expect("uri");
    let file_id = bsl_analysis_v2::FileId(1460);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 360,
        save_cycle_sequence: 120,
        requested_version: 140,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> =
        Arc::from("Procedure Test()\n    UndefinedValue = UnknownIdentifier;\nEndProcedure\n");
    let exact_text_hash = *blake3::hash(exact_text.as_bytes()).as_bytes();
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server
        .diagnostics_generation_v2
        .write()
        .await
        .insert(file_id, key.diagnostics_generation);
    force_current_revision_without_exact_type_index(
        &server,
        file_id,
        &uri,
        exact_text.as_ref(),
        key.requested_version,
    )
    .await;
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
    );
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ReadyInstall,
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
                        save_cycle_sequence: Some(key.save_cycle_sequence),
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::<str>::from(uri.path().to_string()),
                        text: exact_text.clone(),
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
    server
        .latest_detached_diagnostics_ready_artifacts_v2
        .write()
        .await
        .insert(
            file_id,
            crate::server::DetachedDiagnosticsReadyArtifactV2 {
                requested_version: key.requested_version,
                text_hash: exact_text_hash,
                save_cycle_sequence: key.save_cycle_sequence,
                text: exact_text.clone(),
                parse_snapshot: parse_snapshot_for_test(
                    file_id,
                    key.requested_version,
                    exact_text.as_ref(),
                    Vec::new(),
                    true,
                    None,
                ),
                syntax_errors_complete: true,
            },
        );

    let disposition = server
        .try_execute_save_followup_from_ready_artifacts_v2(
            &uri,
            &supersession_key,
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
            None,
            crate::server::core::diagnostics_runtime::ReadyParseSnapshotProbeSlotV2::ZeroBudget,
            Duration::ZERO,
            Instant::now(),
            false,
            false,
            None,
        )
        .await;
    assert!(matches!(
        disposition,
        crate::server::core::diagnostics_runtime::SaveFollowupReadyArtifactsAttemptV2::Executed(
            bsl_runtime::application::DiagnosticsDisposition::Published
        )
    ));

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_ready_snapshot_zero_probe.as_deref(),
        Some("not_ready")
    );
    assert_eq!(
        trace.followup_ready_snapshot_task_state.as_deref(),
        Some("in_flight_same_version")
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("detached_ready_artifacts")
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_detached_ready_artifacts"
        )) > 0,
        "detached ready-artifacts path must export an explicit semantic-path counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24b_diagnostics_save_timeline_ignores_detached_ready_artifacts_from_older_save_cycle() {
    let server = create_diagnostics_save_timeline_test_server();
    prime_server_with_syntax_helper_deps(&server).await;
    let uri =
        Url::parse("file:///p24b-detached-ready-artifacts-older-save-cycle.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(14601);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 3601,
        save_cycle_sequence: 1201,
        requested_version: 1401,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> =
        Arc::from("Procedure Test()\n    UndefinedValue = UnknownIdentifier;\nEndProcedure\n");
    let exact_text_hash = *blake3::hash(exact_text.as_bytes()).as_bytes();
    let older_save_cycle_sequence = key.save_cycle_sequence - 1;
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server
        .diagnostics_generation_v2
        .write()
        .await
        .insert(file_id, key.diagnostics_generation);
    force_current_revision_without_exact_type_index(
        &server,
        file_id,
        &uri,
        exact_text.as_ref(),
        key.requested_version,
    )
    .await;
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
    );
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ReadyInstall,
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
                        save_cycle_sequence: Some(key.save_cycle_sequence),
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::<str>::from(uri.path().to_string()),
                        text: exact_text.clone(),
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
    server
        .latest_detached_diagnostics_ready_artifacts_v2
        .write()
        .await
        .insert(
            file_id,
            crate::server::DetachedDiagnosticsReadyArtifactV2 {
                requested_version: key.requested_version,
                text_hash: exact_text_hash,
                save_cycle_sequence: older_save_cycle_sequence,
                text: exact_text.clone(),
                parse_snapshot: parse_snapshot_for_test(
                    file_id,
                    key.requested_version,
                    exact_text.as_ref(),
                    Vec::new(),
                    true,
                    None,
                ),
                syntax_errors_complete: true,
            },
        );

    let disposition = server
        .try_execute_save_followup_from_ready_artifacts_v2(
            &uri,
            &supersession_key,
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
            None,
            crate::server::core::diagnostics_runtime::ReadyParseSnapshotProbeSlotV2::ZeroBudget,
            Duration::ZERO,
            Instant::now(),
            false,
            false,
            None,
        )
        .await;
    assert!(matches!(
        disposition,
        crate::server::core::diagnostics_runtime::SaveFollowupReadyArtifactsAttemptV2::ProbeMiss(
            crate::server::core::diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::NotReady
        )
    ));

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_ready_snapshot_zero_probe.as_deref(),
        Some("not_ready")
    );
    assert_eq!(
        trace.followup_ready_snapshot_task_state.as_deref(),
        Some("in_flight_same_version")
    );
    assert!(
        trace.followup_semantic_path.is_none(),
        "older detached save-cycle artifact must not be consumed for the newer still-current target, trace={trace:?}"
    );
    let metrics = server.coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_detached_ready_artifacts"
        )),
        0,
        "stale detached save-cycle artifact must not increment the detached semantic-path counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24_diagnostics_save_timeline_continues_still_current_exact_worker_after_relief_timeout() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p24-ready-snapshot-relief-continued-still-current.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(1461);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 361,
        save_cycle_sequence: 121,
        requested_version: 141,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> =
        Arc::from("Procedure Test()\n    UndefinedValue = UnknownIdentifier;\nEndProcedure\n");
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
                        text: exact_text.clone(),
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
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );
    let server_for_ready = server.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(550)).await;
        control.transition_parse_exec_subphase_attribution(
            crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
        );
        let parse_snapshot = parse_snapshot_for_test(
            file_id,
            key.requested_version,
            exact_text.as_ref(),
            Vec::new(),
            false,
            None,
        );
        let ready_state = ReadyParseSnapshotStateV2 {
            text: exact_text,
            parse_snapshot,
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2 {
                parse_exec_ms: Some(4100),
                parse_exec_core_parse_build_ms: Some(4100),
                parse_exec_core_build_pre_parse_setup_ms: None,
                parse_exec_core_build_parser_base_recovery_ms: None,
                parse_exec_core_build_parser_tree_build_ms: Some(4100),
                parse_exec_core_build_exact_ready_snapshot_assembly_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                parse_exec_core_build_tree_cache_install_ms: None,
                parse_exec_optional_cache_enrichment_ms: None,
                post_parse_pre_materialization_ms: None,
                ready_install_ms: Some(1),
                document_symbol_side_work_ms: None,
            },
            program_lowering_summary: None,
        };
        server_for_ready
            .latest_ready_parse_snapshots_v2
            .write()
            .await
            .insert(file_id, ready_state);
        control.materialized_notify.notify_waiters();
    });

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
    assert_eq!(
        disposition,
        Some(bsl_runtime::application::DiagnosticsDisposition::Published)
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("continued_still_current")
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );

    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_ready"
        )) > 0,
        "continued still-current path must export a ready relief probe, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_continued_still_current"
        )) > 0,
        "continued still-current path must export continuation reason counter, counters={counters:?}"
    );
}

#[tokio::test]
async fn p24_diagnostics_save_timeline_continues_still_current_after_bounded_core_parse_build_entry(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse(
        "file:///p24-ready-snapshot-relief-continued-before-first-core-build-checkpoint.bsl",
    )
    .expect("uri");
    let file_id = bsl_analysis_v2::FileId(1462);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 362,
        save_cycle_sequence: 122,
        requested_version: 142,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> =
        Arc::from("Procedure Test()\n    UndefinedValue = UnknownIdentifier;\nEndProcedure\n");
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
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
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
                        text: exact_text.clone(),
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
                control: control.clone(),
                handle: tokio::spawn(async {}),
            },
        );
    let server_for_ready = server.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(550)).await;
        let parse_snapshot = parse_snapshot_for_test(
            file_id,
            key.requested_version,
            exact_text.as_ref(),
            Vec::new(),
            false,
            None,
        );
        let ready_state = ReadyParseSnapshotStateV2 {
            text: exact_text,
            parse_snapshot,
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2 {
                parse_exec_ms: Some(4300),
                parse_exec_core_parse_build_ms: None,
                parse_exec_core_build_pre_parse_setup_ms: None,
                parse_exec_core_build_parser_base_recovery_ms: None,
                parse_exec_core_build_parser_tree_build_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms: None,
                parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                parse_exec_core_build_tree_cache_install_ms: None,
                parse_exec_optional_cache_enrichment_ms: None,
                post_parse_pre_materialization_ms: None,
                ready_install_ms: Some(1),
                document_symbol_side_work_ms: None,
            },
            program_lowering_summary: None,
        };
        server_for_ready
            .latest_ready_parse_snapshots_v2
            .write()
            .await
            .insert(file_id, ready_state);
        control.materialized_notify.notify_waiters();
    });

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
    assert_eq!(
        disposition,
        Some(bsl_runtime::application::DiagnosticsDisposition::Published)
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("continued_still_current")
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
}

#[tokio::test]
async fn p26_diagnostics_save_timeline_records_post_ready_publish_gate_separately_from_apply_lag() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p26-post-ready-publish-gate.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(148),
        diagnostics_generation: 38,
        save_cycle_sequence: 14,
        requested_version: 16,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_followup_wait_state(
        &uri,
        key,
        "pending_publish",
        None,
        Some(Duration::from_millis(23)),
        None,
        None,
        Some("reused"),
        Some("ready_artifacts"),
        Some("snapshot"),
        Some("snapshot_build"),
        None,
    );
    server.record_diagnostics_save_timeline_followup_blocker_reason(
        &uri,
        key,
        "post_ready_publish_gate",
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace.followup_wait_reason.as_deref(),
        Some("pending_publish")
    );
    assert_eq!(
        trace.followup_blocker_reason.as_deref(),
        Some("post_ready_publish_gate")
    );
    assert_eq!(
        trace.followup_semantic_path.as_deref(),
        Some("ready_artifacts")
    );
    assert_eq!(trace.followup_apply_lag_ms, Some(23));
}

#[tokio::test]
async fn p26_diagnostics_save_timeline_relief_valve_does_not_skip_apply_lag_for_late_exact_phase() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p26-relief-apply-lag-late-phase.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(149);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 39,
        save_cycle_sequence: 15,
        requested_version: 17,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 17;\nEndProcedure\n");
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
    server
        .latest_current_revision_handoff_versions_v2
        .write()
        .await
        .insert(file_id, key.requested_version);
    server
        .latest_apply_enqueued_at_v2
        .write()
        .await
        .insert(file_id, Instant::now() - Duration::from_millis(200));
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
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
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
        "late exact phase with apply lag should spend bounded relief wait, disposition={disposition:?}"
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_timed_out")
    );
    assert_eq!(
        trace.followup_ready_snapshot_continuation_reason.as_deref(),
        Some("exhausted_continuation_proof")
    );
    assert_ne!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("skipped_apply_lag")
    );
    let counters = server
        .coordinator
        .observability_metrics()
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object")
        .clone();
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_timed_out"
        )) > 0,
        "late exact phase must export engaged_timed_out instead of skipped_apply_lag, counters={counters:?}"
    );
}

#[tokio::test]
async fn p26_diagnostics_save_timeline_keeps_skipped_apply_lag_for_waiting_exact_phase() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p26-relief-apply-lag-waiting-phase.bsl").expect("uri");
    let file_id = bsl_analysis_v2::FileId(150);
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id,
        diagnostics_generation: 40,
        save_cycle_sequence: 16,
        requested_version: 18,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };
    let exact_text: Arc<str> = Arc::from("Procedure Test()\n    Return 18;\nEndProcedure\n");
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
    server
        .latest_current_revision_handoff_versions_v2
        .write()
        .await
        .insert(file_id, key.requested_version);
    server
        .latest_apply_enqueued_at_v2
        .write()
        .await
        .insert(file_id, Instant::now() - Duration::from_millis(200));
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: key.requested_version,
            text: exact_text.clone(),
        },
    );
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
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
        "waiting-only exact phase should still skip relief on apply lag, disposition={disposition:?}"
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("skipped_apply_lag")
    );
}

#[tokio::test]
async fn p24_diagnostics_save_timeline_preserves_relief_valve_after_terminal_publish() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p24-ready-snapshot-relief-archived-trace.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(147),
        diagnostics_generation: 37,
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
                elapsed_ms: 7,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(4),
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
                publish_wait_ms: Some(0),
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
                elapsed_ms: 3801,
                syntax_work_mode: Some("reused".to_string()),
                semantic_path: Some("ready_artifacts".to_string()),
                semantic_parse_source: Some("snapshot".to_string()),
                semantic_ir_source: Some("snapshot_build".to_string()),
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: None,
                semantic_diagnostics_query_ms: Some(18),
                semantic_diagnostics_inputs_ms: Some(2),
                semantic_diagnostics_parse_result_ms: Some(11),
                semantic_diagnostics_ir_ms: Some(5),
                semantic_diagnostics_collect_ms: Some(2),
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
                publish_wait_ms: Some(0),
                ..Default::default()
            }),
        },
    );
    server.record_diagnostics_save_timeline_followup_relief_valve(
        &uri,
        key,
        "engaged_helped",
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET,
        Some(Duration::from_millis(271)),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(trace.terminal_outcome.as_deref(), Some("published"));
    assert_eq!(
        trace
            .followup_ready_snapshot_relief_valve_outcome
            .as_deref(),
        Some("engaged_helped")
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_budget_ms,
        Some(
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64
        )
    );
    assert_eq!(
        trace.followup_ready_snapshot_relief_valve_elapsed_ms,
        Some(271)
    );
}

#[test]
fn p7_ready_parse_snapshot_probe_wait_decision_classifies_freshness_mismatches() {
    let key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id: bsl_analysis_v2::FileId(91),
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: 7,
        save_cycle_sequence: Some(3),
        requested_version: 11,
    };

    let generation_mismatch = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        None,
        Some(8),
        Some(11),
    );
    assert_eq!(
        generation_mismatch,
        Some(diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::GenerationMismatch)
    );

    let version_mismatch = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        None,
        Some(7),
        Some(12),
    );
    assert_eq!(
        version_mismatch,
        Some(diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::VersionMismatch)
    );
}

#[test]
fn p7_ready_parse_snapshot_probe_wait_decision_classifies_cancellation_and_supersession() {
    let key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id: bsl_analysis_v2::FileId(92),
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: 9,
        save_cycle_sequence: Some(4),
        requested_version: 14,
    };

    let cancelled = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        Some(crate::server::DiagnosticsCancellationReasonV2::ClientCancel),
        Some(9),
        Some(14),
    );
    assert_eq!(
        cancelled,
        Some(diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::Cancelled)
    );

    let superseded = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        Some(crate::server::DiagnosticsCancellationReasonV2::SupersededVersion),
        Some(9),
        Some(14),
    );
    assert_eq!(
        superseded,
        Some(diagnostics_runtime::ReadyParseSnapshotProbeOutcomeV2::Superseded)
    );
}
