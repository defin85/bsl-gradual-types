#[tokio::test]
async fn same_revision_ready_snapshot_waits_for_exact_type_index_before_hover() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "400");

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = concat!(
        "Процедура Тест()\n",
        "    S = Новый Структура;\n",
        "    S.Вставить(\"Идентификатор\", \"A-01\");\n",
        "    ДляHover = S.Идентификатор;\n",
        "КонецПроцедуры\n"
    );
    let uri =
        Url::parse("file:///test_same_revision_ready_waits_for_exact_hover.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    assert!(
        server.analysis_v2.wait_for_file_version(file_id, 1).await,
        "analysis runtime must catch up to opened file version"
    );

    let mut saw_building_while_exact_cold = false;
    let ready_status = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let status = server.snapshot_status_for_uri_v2(&uri).await;
            let exact_ready = server
                .analysis_v2
                .snapshot()
                .await
                .current_type_index_serve_only_ready(file_id)
                .expect("current_type_index_serve_only_ready during ready/exact wait");
            if status.state == SnapshotReadinessStateDto::Ready || status.exact {
                assert!(
                    exact_ready,
                    "snapshot status must not report ready/exact before the exact artifact is actually published, status={status:?}"
                );
                assert_eq!(
                    status.ready_version,
                    Some(1),
                    "ready status must stay pinned to the requested revision, status={status:?}"
                );
                break status;
            }
            if !exact_ready {
                saw_building_while_exact_cold = true;
                assert!(
                    status.state != SnapshotReadinessStateDto::Ready && !status.exact,
                    "same-revision status must remain non-ready while exact artifact is still cold, status={status:?}"
                );
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("same-revision ready/exact publish must complete for small hover fixture");
    assert!(
        saw_building_while_exact_cold,
        "test must observe the transient parse-ready/exact-cold window before the fix closes it, status={ready_status:?}"
    );

    let hover_position = find_utf16_position_at_marker_tail(fixture, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position)
        .await
        .expect("hover should succeed once same-revision ready/exact publish completes");
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose typed structure field info after same-revision ready/exact publish, hover={hover_text}, status={ready_status:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn detached_ready_artifact_does_not_weaken_hover_fail_closed_gate() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "400");

    let fixture = concat!(
        "Процедура Тест()\n",
        "    S = Новый Структура;\n",
        "    S.Вставить(\"Идентификатор\", \"A-01\");\n",
        "    ДляHover = S.Идентификатор;\n",
        "КонецПроцедуры\n"
    );
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///detached-ready-artifact-hover-fail-closed.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let exact_ready_before_hover = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready before detached hover probe");
    assert!(
        !exact_ready_before_hover,
        "test setup must keep the current exact type index unpublished before hover"
    );

    let text: Arc<str> = Arc::from(fixture.to_string());
    let text_hash = *blake3::hash(text.as_bytes()).as_bytes();
    server
        .latest_detached_diagnostics_ready_artifacts_v2
        .write()
        .await
        .insert(
            file_id,
            crate::server::DetachedDiagnosticsReadyArtifactV2 {
                requested_version: 1,
                text_hash,
                save_cycle_sequence: 1,
                text: text.clone(),
                parse_snapshot: parse_snapshot_for_test(file_id, 1, text.as_ref(), Vec::new(), true, None),
                syntax_errors_complete: true,
            },
        );

    let hover_position = find_utf16_position_at_marker_tail(fixture, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position).await;
    assert!(
        hover_text.is_none(),
        "hover must remain fail-closed while only detached diagnostics-ready artifacts exist"
    );

    let exact_ready_after_hover = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after detached hover probe");
    assert!(
        !exact_ready_after_hover,
        "detached diagnostics-ready artifacts must not mark canonical hover exact readiness"
    );

    drain_task.abort();
}

#[tokio::test]
async fn snapshot_status_request_reports_exact_ready_for_matching_snapshot() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///snapshot-status-ready.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let text: Arc<str> = Arc::from("Procedure Test()\nEndProcedure\n");

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 7);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 7,
            text: text.clone(),
        },
    );
    server.latest_ready_parse_snapshots_v2.write().await.insert(
        file_id,
        ReadyParseSnapshotStateV2 {
            text: text.clone(),
            parse_snapshot: parse_snapshot_for_test(file_id, 7, text.as_ref(), vec![], true, None),
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2::default(),
            program_lowering_summary: None,
        },
    );

    let status = server
        .handle_get_snapshot_status(crate::types::GetSnapshotStatusRequest {
            uri: uri.to_string(),
        })
        .await
        .expect("snapshot status request");
    assert_eq!(status.uri.as_deref(), Some(uri.as_str()));
    assert_eq!(status.requested_version, Some(7));
    assert_eq!(status.ready_version, Some(7));
    assert_eq!(status.state, SnapshotReadinessStateDto::Ready);
    assert!(status.exact, "matching ready snapshot must be exact");
    assert_eq!(status.task_state, SnapshotTaskStateDto::ReadySameRevision);
    assert!(status.updated_at_ms > 0);

    harness.shutdown().await;
}

#[tokio::test]
async fn snapshot_status_request_reports_building_for_matching_inflight_worker() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///snapshot-status-building.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let text: Arc<str> = Arc::from("Procedure Test()\nEndProcedure\n");
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());
    control.phase.store(
        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Parsing as u8,
        Ordering::SeqCst,
    );

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 3);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 3,
            text: text.clone(),
        },
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
                        requested_version: 3,
                        text_hash: *blake3::hash(text.as_bytes()).as_bytes(),
                        save_cycle_sequence: None,
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::from(uri.path().to_string()),
                        text: text.clone(),
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

    let status = server.snapshot_status_for_uri_v2(&uri).await;
    assert_eq!(status.state, SnapshotReadinessStateDto::Building);
    assert!(
        !status.exact,
        "in-flight worker must not claim exact readiness"
    );
    assert_eq!(
        status.task_state,
        SnapshotTaskStateDto::InFlightSameRevision
    );
    assert_eq!(status.phase, Some(SnapshotPhaseDto::Parsing));

    harness.shutdown().await;
}

#[tokio::test]
async fn prime_exact_type_index_request_schedules_same_revision_exact_warmup_without_completion() {
    const FIXTURE: &str = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";
    let (mut service, drain_task, server, uri, file_id) =
        open_lsp_fixture_with_snapshot(FIXTURE, "file:///prime-exact-type-index-request.bsl").await;

    force_current_revision_without_exact_type_index(&server, file_id, &uri, FIXTURE, 2).await;

    let exact_ready_before = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready before prime request");
    assert!(
        !exact_ready_before,
        "test setup must start with current-revision exact type index unpublished"
    );

    let response = server
        .handle_prime_exact_type_index(crate::types::PrimeExactTypeIndexRequest {
            uri: uri.to_string(),
            requested_version: Some(2),
            reason: Some("test_exact_warmup".to_string()),
        })
        .await
        .expect("prime exact type index request");
    assert!(
        response.accepted,
        "prime exact type index request must be accepted for an open current-revision document"
    );
    assert!(
        !response.already_ready,
        "test setup must exercise the cold exact-index path"
    );
    assert_eq!(response.observed_version, Some(2));
    assert!(
        matches!(
            response.action.as_str(),
            "promoted" | "joined" | "scheduled"
        ),
        "prime exact type index request must report a scheduling action, response={response:?}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;
    let exact_ready_after = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after prime request");
    assert!(
        exact_ready_after,
        "prime exact type index request must publish current-revision exact type index without requiring completion first"
    );

    let hover_position = find_utf16_position_at_marker_tail(FIXTURE, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_optional_at(&mut service, &uri, hover_position)
        .await
        .expect("hover should succeed after explicit exact-index prime");
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose typed structure field info after explicit exact-index prime, hover={hover_text}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn snapshot_status_updated_at_is_monotonic_across_building_to_ready_transition() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///snapshot-status-monotonic.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let text: Arc<str> = Arc::from("Procedure Test()\nEndProcedure\n");

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 11);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 11,
            text: text.clone(),
        },
    );
    let waiting_control =
        Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());
    waiting_control.phase.store(
        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Waiting as u8,
        Ordering::SeqCst,
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
                        requested_version: 11,
                        text_hash: *blake3::hash(text.as_bytes()).as_bytes(),
                        save_cycle_sequence: Some(7),
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
                        path: Arc::from(uri.path().to_string()),
                        text: text.clone(),
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
                control: waiting_control,
                handle: tokio::spawn(async {}),
            },
        );

    let building = server.snapshot_status_for_uri_v2(&uri).await;
    assert_eq!(building.state, SnapshotReadinessStateDto::Building);

    server
        .background_parse_snapshot_apply_tasks_v2
        .lock()
        .await
        .remove(&file_id);
    server.latest_ready_parse_snapshots_v2.write().await.insert(
        file_id,
        ReadyParseSnapshotStateV2 {
            text: text.clone(),
            parse_snapshot: parse_snapshot_for_test(file_id, 11, text.as_ref(), vec![], true, None),
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidSave,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2::default(),
            program_lowering_summary: None,
        },
    );

    let ready = server.snapshot_status_for_uri_v2(&uri).await;
    assert_eq!(ready.state, SnapshotReadinessStateDto::Ready);
    assert!(
        ready.updated_at_ms > building.updated_at_ms,
        "updatedAtMs must increase across a real state transition"
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn snapshot_status_live_notifications_coalesce_phase_only_building_transitions() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;

    let uri = Url::parse("file:///snapshot-status-phase-coalesce.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let text: Arc<str> = Arc::from("Procedure Test()\nEndProcedure\n");
    let control = Arc::new(crate::server::BackgroundParseSnapshotApplyTaskControlV2::new());

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 21);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 21,
            text: text.clone(),
        },
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
                        requested_version: 21,
                        text_hash: *blake3::hash(text.as_bytes()).as_bytes(),
                        save_cycle_sequence: None,
                        source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
                        path: Arc::from(uri.path().to_string()),
                        text: text.clone(),
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

    control.phase.store(
        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Waiting as u8,
        Ordering::SeqCst,
    );
    server.refresh_snapshot_status_v2(file_id).await;

    let building =
        wait_for_snapshot_status_notification(&mut harness, Duration::from_secs(1)).await;
    assert_eq!(building.state, SnapshotReadinessStateDto::Building);
    assert_eq!(building.phase, Some(SnapshotPhaseDto::Waiting));

    control.phase.store(
        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Parsing as u8,
        Ordering::SeqCst,
    );
    server.refresh_snapshot_status_v2(file_id).await;
    assert_no_snapshot_status_notification(&mut harness, Duration::from_millis(100)).await;

    control.phase.store(
        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Materializing as u8,
        Ordering::SeqCst,
    );
    server.refresh_snapshot_status_v2(file_id).await;
    assert_no_snapshot_status_notification(&mut harness, Duration::from_millis(100)).await;

    server
        .background_parse_snapshot_apply_tasks_v2
        .lock()
        .await
        .remove(&file_id);
    server.latest_ready_parse_snapshots_v2.write().await.insert(
        file_id,
        ReadyParseSnapshotStateV2 {
            text: text.clone(),
            parse_snapshot: parse_snapshot_for_test(file_id, 21, text.as_ref(), vec![], true, None),
            source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange,
            syntax_errors_complete: true,
            phase_attribution: crate::server::ReadyParseSnapshotPhaseAttributionV2::default(),
            program_lowering_summary: None,
        },
    );
    server.refresh_snapshot_status_v2(file_id).await;

    let ready = wait_for_snapshot_status_notification(&mut harness, Duration::from_secs(1)).await;
    assert_eq!(ready.state, SnapshotReadinessStateDto::Ready);
    assert!(ready.updated_at_ms > building.updated_at_ms);

    harness.shutdown().await;
}

#[tokio::test]
async fn snapshot_status_request_reports_shadow_only_when_only_shadow_state_is_current() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///snapshot-status-shadow-only.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 5);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 5,
            text: Arc::from("Procedure Test()\n    x = 1;\nEndProcedure\n"),
        },
    );

    let status = server.snapshot_status_for_uri_v2(&uri).await;
    assert_eq!(status.state, SnapshotReadinessStateDto::ShadowOnly);
    assert!(!status.exact);
    assert_eq!(status.task_state, SnapshotTaskStateDto::Absent);
    assert_eq!(status.requested_version, Some(5));
    assert_eq!(status.ready_version, None);

    harness.shutdown().await;
}

#[tokio::test]
async fn snapshot_status_request_reports_failed_when_last_build_aborted() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    let uri = Url::parse("file:///snapshot-status-failed.bsl").expect("uri");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 17);
    server.latest_snapshot_failures_v2.write().await.insert(
        file_id,
        SnapshotBuildFailureStateV2 {
            requested_version: 17,
            reason: Arc::from("build_snapshot_aborted"),
        },
    );

    let status = server.snapshot_status_for_uri_v2(&uri).await;
    assert_eq!(status.state, SnapshotReadinessStateDto::Failed);
    assert!(!status.exact);
    assert_eq!(status.task_state, SnapshotTaskStateDto::Absent);
    assert_eq!(status.requested_version, Some(17));
    assert_eq!(
        status.fallback_reason.as_deref(),
        Some("build_snapshot_aborted")
    );

    harness.shutdown().await;
}
