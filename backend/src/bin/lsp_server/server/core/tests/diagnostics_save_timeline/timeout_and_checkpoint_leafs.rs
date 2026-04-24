#[tokio::test]
async fn p23_diagnostics_save_timeline_reports_parse_exec_timeout_phase_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p23-ready-snapshot-timeout-parse.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(141),
        diagnostics_generation: 31,
        save_cycle_sequence: 7,
        requested_version: 9,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("parse timeout phase attribution");
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
        trace.followup_ready_snapshot_wait_probe.as_deref(),
        Some("timeout")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("before_first_parse_exec_subphase")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_timeout_phase_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace.followup_ready_snapshot_dominant_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_post_parse_pre_materialization_ms,
        None
    );
}

#[tokio::test]
async fn p27_diagnostics_save_timeline_reports_parse_exec_core_subphase_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p27-ready-snapshot-timeout-parse-core.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(144),
        diagnostics_generation: 34,
        save_cycle_sequence: 10,
        requested_version: 12,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control
        .transition_phase_attribution(crate::server::ReadyParseSnapshotAttributionPhaseV2::Waiting);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("parse core timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("before_first_core_build_checkpoint")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_parse_build_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_dominant_subphase
            .as_deref(),
        Some("core_parse_build")
    );
}

#[tokio::test]
async fn p27_diagnostics_save_timeline_reports_pre_parse_setup_checkpoint_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p27-ready-snapshot-timeout-pre-parse-setup.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1442),
        diagnostics_generation: 342,
        save_cycle_sequence: 102,
        requested_version: 112,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::PreParseSetup,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("pre-parse-setup timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("pre_parse_setup")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("pre_parse_setup")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint
            .as_deref(),
        Some("pre_parse_setup")
    );
}

#[tokio::test]
async fn p27_diagnostics_save_timeline_reports_parser_base_recovery_checkpoint_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p27-ready-snapshot-timeout-parser-base-recovery.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(1443),
        diagnostics_generation: 343,
        save_cycle_sequence: 103,
        requested_version: 113,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ParserBaseRecovery,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("parser-base-recovery timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("parser_base_recovery")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("parser_base_recovery")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint
            .as_deref(),
        Some("parser_base_recovery")
    );
}

#[tokio::test]
async fn p28_diagnostics_save_timeline_reports_exact_ready_snapshot_assembly_pre_checkpoint_leaf_for_exact_worker(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p28-ready-snapshot-timeout-assembly-pre-checkpoint.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(2441),
        diagnostics_generation: 441,
        save_cycle_sequence: 201,
        requested_version: 221,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

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
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("assembly pre-checkpoint timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("before_first_exact_ready_snapshot_assembly_checkpoint")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("exact_ready_snapshot_assembly")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint,
        None
    );
}

#[tokio::test]
async fn p28_diagnostics_save_timeline_reports_core_build_checkpoint_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p28-ready-snapshot-timeout-core-build-checkpoint.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(244),
        diagnostics_generation: 44,
        save_cycle_sequence: 20,
        requested_version: 22,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ParserTreeBuild,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("core-build checkpoint timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("parser_tree_build")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("parser_tree_build")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint
            .as_deref(),
        Some("parser_tree_build")
    );
}

#[tokio::test]
async fn p29_diagnostics_save_timeline_reports_exact_ready_snapshot_assembly_checkpoint_for_exact_worker(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p29-ready-snapshot-timeout-assembly-checkpoint.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(245),
        diagnostics_generation: 45,
        save_cycle_sequence: 21,
        requested_version: 23,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

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
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("exact-ready assembly timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("program_lowering")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("exact_ready_snapshot_assembly")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint
            .as_deref(),
        Some("program_lowering")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms
        .is_some_and(|value| value > 0));
    assert!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms
            .is_none()
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint
            .as_deref(),
        Some("program_lowering")
    );
}

#[tokio::test]
async fn p30_diagnostics_save_timeline_reports_publishable_artifact_packaging_checkpoint_for_exact_worker(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri =
        Url::parse("file:///p30-ready-snapshot-timeout-packaging-checkpoint.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(246),
        diagnostics_generation: 46,
        save_cycle_sequence: 22,
        requested_version: 24,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

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
    tokio::time::sleep(Duration::from_millis(10)).await;
    control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("publishable-artifact-packaging timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("parse_exec")
    );
    assert_eq!(
        trace.followup_ready_snapshot_timeout_leaf.as_deref(),
        Some("publishable_artifact_packaging")
    );
    assert!(trace
        .followup_ready_snapshot_timeout_leaf_elapsed_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_timeout_subphase
            .as_deref(),
        Some("core_parse_build")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
            .as_deref(),
        Some("exact_ready_snapshot_assembly")
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint
            .as_deref(),
        Some("publishable_artifact_packaging")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint
            .as_deref(),
        Some("publishable_artifact_packaging")
    );
}

#[tokio::test]
async fn p31_diagnostics_save_timeline_repeated_probe_snapshots_keep_exact_ready_snapshot_view_coherent(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p31-ready-snapshot-repeated-probes-coherent.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(247),
        diagnostics_generation: 47,
        save_cycle_sequence: 23,
        requested_version: 25,
    };
    let timeout_control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());
    let ready_control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);

    timeout_control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    timeout_control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    timeout_control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly,
    );
    timeout_control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    timeout_control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::PublishableArtifactPackaging,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let timeout_attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &timeout_control.phase_attribution_snapshot(),
            true,
        )
        .expect("timeout attribution");
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some("not_ready"),
        Some("timeout"),
        Some("in_flight_same_version"),
        Some(true),
        Some(timeout_attribution),
    );

    ready_control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    ready_control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    ready_control.transition_core_build_checkpoint_attribution(
        crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::ExactReadySnapshotAssembly,
    );
    ready_control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering,
    );
    tokio::time::sleep(Duration::from_millis(8)).await;
    let ready_attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &ready_control.phase_attribution_snapshot(),
            false,
        )
        .expect("ready attribution");
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some("ready"),
        Some("ready"),
        Some("ready_same_version"),
        Some(true),
        Some(ready_attribution),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint
            .as_deref(),
        Some("publishable_artifact_packaging")
    );
    assert!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms
            .is_some_and(|value| value > 0)
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
        "latest coherent attribution view must replace stale aggregate/slice maxima"
    );
    assert!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms
            .is_none(),
        "stale packaging slice from an older probe must not leak into the final trace"
    );
    assert_eq!(
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint
            .as_deref(),
        Some("program_lowering")
    );
}

#[tokio::test]
async fn p31_diagnostics_save_timeline_reentered_program_lowering_keeps_program_conversion_coherent(
) {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p31-ready-snapshot-reentered-program-lowering-coherent.bsl")
        .expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(248),
        diagnostics_generation: 48,
        save_cycle_sequence: 24,
        requested_version: 26,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

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
    tokio::time::sleep(Duration::from_millis(20)).await;
    control.transition_assembly_checkpoint_attribution(
        crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;

    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("program-lowering reentry attribution");
    assert_eq!(
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
        "mixed current/completed program_lowering state must not export incoherent aggregate"
    );

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
        trace
            .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
        "timeline must keep the coherent aggregate view from reentered program_lowering snapshots"
    );
}

#[tokio::test]
async fn p31_diagnostics_save_timeline_reentered_core_parse_build_keeps_first_bounded_elapsed() {
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;

    let first_attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("first core-parse-build attribution");
    let first_elapsed = first_attribution
        .timeout_leaf_elapsed_ms
        .expect("first core-parse-build timeout leaf elapsed");

    control.transition_parse_exec_subphase_attribution(
        crate::server::ReadyParseSnapshotParseExecSubphaseV2::CoreParseBuild,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;

    let second_attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("second core-parse-build attribution");
    assert_eq!(
        second_attribution.timeout_leaf,
        Some("before_first_core_build_checkpoint")
    );
    assert_eq!(
        second_attribution.parse_exec_timeout_subphase,
        Some("core_parse_build")
    );
    assert!(
        second_attribution
            .timeout_leaf_elapsed_ms
            .is_some_and(|value| value > first_elapsed),
        "re-entering the same parse_exec subphase must not reset the first bounded elapsed time"
    );
    assert!(
        second_attribution
            .parse_exec_core_parse_build_ms
            .is_some_and(|value| value > first_elapsed),
        "core_parse_build attribution must keep accumulating across repeated identical callbacks"
    );
}

#[tokio::test]
async fn p23_diagnostics_save_timeline_reports_post_parse_timeout_phase_for_exact_worker() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p23-ready-snapshot-timeout-post-parse.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(142),
        diagnostics_generation: 32,
        save_cycle_sequence: 8,
        requested_version: 10,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            true,
        )
        .expect("post-parse timeout phase attribution");
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
        trace.followup_ready_snapshot_timeout_phase.as_deref(),
        Some("post_parse_pre_materialization")
    );
    assert!(trace
        .followup_ready_snapshot_parse_exec_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_post_parse_pre_materialization_ms
        .is_some_and(|value| value > 0));
    assert_eq!(
        trace.followup_ready_snapshot_dominant_phase.as_deref(),
        Some("post_parse_pre_materialization")
    );
}

#[tokio::test]
async fn p23_ready_snapshot_phase_attribution_separates_document_symbol_side_work() {
    let server = create_diagnostics_save_timeline_test_server();
    let uri = Url::parse("file:///p23-ready-snapshot-document-symbol-side-work.bsl").expect("uri");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(143),
        diagnostics_generation: 33,
        save_cycle_sequence: 9,
        requested_version: 11,
    };
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ParseExec,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::ReadyInstall,
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    let _ = control.finish_phase_attribution();
    control.transition_phase_attribution(
        crate::server::ReadyParseSnapshotAttributionPhaseV2::DocumentSymbolSideWork,
    );
    tokio::time::sleep(Duration::from_millis(10)).await;
    let attribution =
        diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
            &control.phase_attribution_snapshot(),
            false,
        )
        .expect("document-symbol side-work attribution");
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some("ready"),
        None,
        Some("ready_same_version"),
        Some(true),
        Some(attribution),
    );

    let trace = diagnostics_save_timeline_trace_for_test(&server, &uri, key).await;
    assert!(trace
        .followup_ready_snapshot_ready_install_ms
        .is_some_and(|value| value > 0));
    assert!(trace
        .followup_ready_snapshot_document_symbol_side_work_ms
        .is_some_and(|value| value > 0));
    assert_eq!(trace.followup_ready_snapshot_timeout_phase, None);
    assert_eq!(
        trace.followup_ready_snapshot_dominant_phase.as_deref(),
        Some("document_symbol_side_work")
    );
}
