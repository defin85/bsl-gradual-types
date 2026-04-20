#[tokio::test]
async fn p33_current_revision_exact_prewarm_shares_ir_singleflight_with_request_path() {
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

    const FIXTURE_V1: &str = "Процедура Тест()\n    Значение = Новый Структура;\nКонецПроцедуры\n";
    const FIXTURE_V2: &str =
        "Процедура Тест()\n    Значение = Новый Структура(\"Поле\", 1);\nКонецПроцедуры\n";
    const LEADER_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_leader_query_kind_ir";
    const SHARED_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_shared_query_kind_ir";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "500");
    let _ir_build_delay_guard = EnvVarGuard::set("BSL_TEST_ANALYSIS_IR_BUILD_DELAY_MS", "250");

    let (_service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        FIXTURE_V1,
        "file:///test_p33_current_revision_exact_ir_singleflight_reuse.bsl",
    )
    .await;

    let metrics_before = server.coordinator.observability_metrics();
    let counters_before = metrics_before
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before.counters object");
    let leader_before = read_u64_metric(counters_before.get(LEADER_IR_METRIC));
    let shared_before = read_u64_metric(counters_before.get(SHARED_IR_METRIC));

    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: FIXTURE_V2.to_string(),
            }],
        })
        .await;
    server.cancel_type_index_precompute_v2(file_id).await;

    assert!(
        server
            .analysis_v2
            .wait_for_file_version_for_operation(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                2,
            )
            .await,
        "runtime must observe current revision before request-side IR query joins prewarm"
    );

    let leader_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        let metrics = server.coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        if read_u64_metric(counters.get(LEADER_IR_METRIC)) >= leader_before.saturating_add(1) {
            break;
        }
        if tokio::time::Instant::now() >= leader_deadline {
            panic!(
                "current-revision prewarm did not become IR singleflight leader in time, counters={counters:?}"
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let completion_snapshot = server
        .analysis_v2
        .completion_current_revision_snapshot_for_origin_and_operation(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            bsl_runtime::application::SemanticOperation::Completion,
        )
        .await;
    let analysis = completion_snapshot.analysis;
    assert_eq!(
        analysis.file_version(file_id).expect("file_version"),
        Some(2),
        "request-side snapshot must target same current revision as prewarm"
    );
    let deps_id = completion_snapshot.deps_id;
    let settings_id = analysis.settings_id().expect("settings id");
    let coordinator = server.coordinator.clone();
    let request_ir = tokio::task::spawn_blocking(move || {
        let context = bsl_runtime::application::ExecutionContext {
            origin: bsl_runtime::application::ObservabilityOrigin::Lsp,
            operation: bsl_runtime::application::SemanticOperation::Completion,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(2),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: bsl_runtime::application::ExecutionSettings {
                settings_id,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::Full,
            },
            cancellation: bsl_runtime::application::CancellationPolicy::RespectClientAbort,
        };

        bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(coordinator.as_ref()),
            file_id,
        )
    })
    .await
    .expect("request ir join")
    .expect("request ir singleflight")
    .expect("request ir result");
    assert!(
        !request_ir.nodes.is_empty(),
        "request-side exact IR query must return a semantic program"
    );

    let ready_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        if server
            .analysis_v2
            .snapshot()
            .await
            .current_completion_head_ready(file_id)
            .expect("current_completion_head_ready")
        {
            break;
        }
        if tokio::time::Instant::now() >= ready_deadline {
            panic!("current-revision head artifact did not become ready after shared IR flight");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let metrics_after = server.coordinator.observability_metrics();
    let counters_after = metrics_after
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_after.counters object");
    let leader_delta =
        read_u64_metric(counters_after.get(LEADER_IR_METRIC)).saturating_sub(leader_before);
    let shared_delta =
        read_u64_metric(counters_after.get(SHARED_IR_METRIC)).saturating_sub(shared_before);

    assert_eq!(
        leader_delta, 1,
        "same-revision prewarm/request overlap must produce exactly one IR singleflight leader, counters={counters_after:?}"
    );
    assert_eq!(
        shared_delta, 1,
        "request path must attach as shared follower to current-revision prewarm instead of starting duplicate IR compute, counters={counters_after:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_current_revision_exact_prewarm_reuses_request_started_ir_singleflight() {
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

    const FIXTURE_V1: &str = "Процедура Тест()\n    Значение = Новый Структура;\nКонецПроцедуры\n";
    const FIXTURE_V2: &str =
        "Процедура Тест()\n    Значение = Новый Структура(\"Поле\", 1);\nКонецПроцедуры\n";
    const LEADER_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_leader_query_kind_ir";
    const SHARED_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_shared_query_kind_ir";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _ir_build_delay_guard = EnvVarGuard::set("BSL_TEST_ANALYSIS_IR_BUILD_DELAY_MS", "250");

    let (_service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        FIXTURE_V1,
        "file:///test_p33_request_started_ir_singleflight_reuse.bsl",
    )
    .await;

    let metrics_before = server.coordinator.observability_metrics();
    let counters_before = metrics_before
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before.counters object");
    let leader_before = read_u64_metric(counters_before.get(LEADER_IR_METRIC));
    let shared_before = read_u64_metric(counters_before.get(SHARED_IR_METRIC));

    let file_path: Arc<str> = Arc::from(
        uri.to_file_path()
            .expect("fixture file path")
            .to_string_lossy()
            .to_string(),
    );
    let fixture_v2: Arc<str> = Arc::from(FIXTURE_V2);
    server.analysis_v2.apply_changes_interactive(
        bsl_runtime::application::ObservabilityOrigin::Lsp,
        vec![bsl_analysis_v2::Change::SetFileWithSnapshot {
            file_id,
            text: fixture_v2.clone(),
            version: 2,
            path: file_path,
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                2,
                fixture_v2.as_ref(),
                vec![],
                true,
                None,
            ),
        }],
    );
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 2);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 2,
            text: fixture_v2,
        },
    );

    assert!(
        server
            .analysis_v2
            .wait_for_file_version_for_operation(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                2,
            )
            .await,
        "runtime must observe current revision before request-side IR query becomes leader"
    );

    let completion_snapshot = server
        .analysis_v2
        .completion_current_revision_snapshot_for_origin_and_operation(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            bsl_runtime::application::SemanticOperation::Completion,
        )
        .await;
    let analysis = completion_snapshot.analysis;
    assert_eq!(
        analysis.file_version(file_id).expect("file_version"),
        Some(2),
        "request-side snapshot must target same current revision as delayed prewarm"
    );
    let deps_id = completion_snapshot.deps_id;
    let settings_id = analysis.settings_id().expect("settings id");
    let coordinator = server.coordinator.clone();
    let request_ir = tokio::task::spawn_blocking(move || {
        let context = bsl_runtime::application::ExecutionContext {
            origin: bsl_runtime::application::ObservabilityOrigin::Lsp,
            operation: bsl_runtime::application::SemanticOperation::Completion,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(2),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: bsl_runtime::application::ExecutionSettings {
                settings_id,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::Full,
            },
            cancellation: bsl_runtime::application::CancellationPolicy::RespectClientAbort,
        };

        bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(coordinator.as_ref()),
            file_id,
        )
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let prewarm_snapshot = server
        .analysis_v2
        .completion_current_revision_snapshot_for_origin_and_operation(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            bsl_runtime::application::SemanticOperation::Completion,
        )
        .await;
    let prewarm_server = server.clone();
    let prewarm_task = tokio::spawn(async move {
        prewarm_server
            .run_completion_exact_ir_singleflight_prewarm_v2(
                prewarm_snapshot.analysis,
                file_id,
                bsl_runtime::application::CpuWorkClass::Background,
                false,
            )
            .await;
    });

    let shared_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        let metrics = server.coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let leader_delta =
            read_u64_metric(counters.get(LEADER_IR_METRIC)).saturating_sub(leader_before);
        let shared_delta =
            read_u64_metric(counters.get(SHARED_IR_METRIC)).saturating_sub(shared_before);
        if leader_delta >= 1 && shared_delta >= 1 {
            break;
        }
        if tokio::time::Instant::now() >= shared_deadline {
            panic!(
                "delayed prewarm did not reuse request-started IR flight in time, counters={counters:?}"
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let request_ir = request_ir
        .await
        .expect("request ir join")
        .expect("request ir singleflight");
    prewarm_task.await.expect("prewarm task join");
    if let Some(request_ir) = request_ir.as_ref() {
        assert!(
            !request_ir.nodes.is_empty(),
            "request-started exact IR query must return a non-empty semantic program when the follower flight resolves directly"
        );
    }

    let ready_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        if server
            .analysis_v2
            .snapshot()
            .await
            .current_completion_head_ready(file_id)
            .expect("current_completion_head_ready")
        {
            break;
        }
        if tokio::time::Instant::now() >= ready_deadline {
            panic!(
                "current-revision head artifact did not become ready after request-started shared IR flight"
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let metrics_after = server.coordinator.observability_metrics();
    let counters_after = metrics_after
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_after.counters object");
    let leader_delta =
        read_u64_metric(counters_after.get(LEADER_IR_METRIC)).saturating_sub(leader_before);
    let shared_delta =
        read_u64_metric(counters_after.get(SHARED_IR_METRIC)).saturating_sub(shared_before);

    assert_eq!(
        leader_delta, 1,
        "request-started same-revision overlap must produce exactly one IR singleflight leader, counters={counters_after:?}"
    );
    assert_eq!(
        shared_delta, 1,
        "delayed current-revision prewarm must attach as shared follower to request-started IR flight instead of starting duplicate compute, counters={counters_after:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_request_side_ir_singleflight_remains_leader_when_current_revision_prewarm_starts_first(
) {
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

    const FIXTURE_V1: &str = "Процедура Тест()\n    Значение = Новый Структура;\nКонецПроцедуры\n";
    const FIXTURE_V2: &str =
        "Процедура Тест()\n    Значение = Новый Структура(\"Поле\", 1);\nКонецПроцедуры\n";
    const LEADER_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_leader_query_kind_ir";
    const SHARED_IR_METRIC: &str = "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_shared_query_kind_ir";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _ir_build_delay_guard = EnvVarGuard::set("BSL_TEST_ANALYSIS_IR_BUILD_DELAY_MS", "250");

    let (_service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        FIXTURE_V1,
        "file:///test_p33_prewarm_first_request_leader.bsl",
    )
    .await;

    let file_path: Arc<str> = Arc::from(
        uri.to_file_path()
            .expect("fixture file path")
            .to_string_lossy()
            .to_string(),
    );
    let fixture_v2: Arc<str> = Arc::from(FIXTURE_V2);
    server.analysis_v2.apply_changes_interactive(
        bsl_runtime::application::ObservabilityOrigin::Lsp,
        vec![bsl_analysis_v2::Change::SetFileWithSnapshot {
            file_id,
            text: fixture_v2.clone(),
            version: 2,
            path: file_path,
            parse_snapshot: parse_snapshot_for_test(
                file_id,
                2,
                fixture_v2.as_ref(),
                vec![],
                true,
                None,
            ),
        }],
    );
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, 2);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version: 2,
            text: fixture_v2,
        },
    );

    assert!(
        server
            .analysis_v2
            .wait_for_file_version_for_operation(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                bsl_runtime::application::SemanticOperation::Completion,
                file_id,
                2,
            )
            .await,
        "runtime must observe current revision before prewarm-first overlap starts"
    );

    let metrics_before = server.coordinator.observability_metrics();
    let counters_before = metrics_before
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before.counters object");
    let leader_before = read_u64_metric(counters_before.get(LEADER_IR_METRIC));
    let shared_before = read_u64_metric(counters_before.get(SHARED_IR_METRIC));

    let prewarm_snapshot = server
        .analysis_v2
        .completion_current_revision_snapshot_for_origin_and_operation(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            bsl_runtime::application::SemanticOperation::Completion,
        )
        .await;
    let prewarm_server = server.clone();
    let prewarm_task = tokio::spawn(async move {
        prewarm_server
            .run_completion_exact_ir_singleflight_prewarm_v2(
                prewarm_snapshot.analysis,
                file_id,
                bsl_runtime::application::CpuWorkClass::Background,
                false,
            )
            .await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let request_snapshot = server
        .analysis_v2
        .completion_current_revision_snapshot_for_origin_and_operation(
            bsl_runtime::application::ObservabilityOrigin::Lsp,
            bsl_runtime::application::SemanticOperation::Hover,
        )
        .await;
    let analysis = request_snapshot.analysis;
    let deps_id = request_snapshot.deps_id;
    let settings_id = analysis.settings_id().expect("settings id");
    let coordinator = server.coordinator.clone();
    let request_ir = tokio::task::spawn_blocking(move || {
        let context = bsl_runtime::application::ExecutionContext {
            origin: bsl_runtime::application::ObservabilityOrigin::Lsp,
            operation: bsl_runtime::application::SemanticOperation::Hover,
            completion_mode: None,
            completion_large_churn_active: false,
            file_id,
            min_file_version: Some(2),
            expected_deps_id: Some(deps_id),
            flow_sensitive: false,
            settings: bsl_runtime::application::ExecutionSettings {
                settings_id,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::Full,
            },
            cancellation: bsl_runtime::application::CancellationPolicy::RespectClientAbort,
        };

        bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(coordinator.as_ref()),
            file_id,
        )
    });

    let request_ir = request_ir
        .await
        .expect("request ir join")
        .expect("request ir singleflight")
        .expect("request ir result");
    prewarm_task.await.expect("prewarm task join");
    assert!(
        !request_ir.nodes.is_empty(),
        "request-side IR query must still return a semantic program when prewarm started first"
    );

    let metrics_after = server.coordinator.observability_metrics();
    let counters_after = metrics_after
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_after.counters object");
    let leader_delta =
        read_u64_metric(counters_after.get(LEADER_IR_METRIC)).saturating_sub(leader_before);
    let shared_delta =
        read_u64_metric(counters_after.get(SHARED_IR_METRIC)).saturating_sub(shared_before);

    assert_eq!(
        leader_delta, 1,
        "request-side IR must remain the singleflight leader when current-revision prewarm starts first, counters={counters_after:?}"
    );
    assert_eq!(
        shared_delta, 0,
        "passive current-revision prewarm must not capture later interactive IR into a shared follower wait, counters={counters_after:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_head_and_exact_resolve_keep_candidate_id_stable_for_same_revision() {
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

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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

    let uri = Url::parse("file:///test_p33_completion_resolve_candidate_parity.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");
    server.sync_v2_globals().await;
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let head_items = lsp_completion_items_with_request(
        &mut service,
        14001,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let head_item = head_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("head completion item for Добавить");
    let head_candidate_id = head_item
        .data
        .as_ref()
        .and_then(|value| value.get("candidate_id"))
        .cloned()
        .expect("head completion candidate_id");

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let resolved_head_item =
        lsp_completion_resolve_item_with_request(&mut service, 14002, head_item.clone()).await;
    assert!(
        resolved_head_item.detail != head_item.detail
            || resolved_head_item.documentation != head_item.documentation,
        "head item must gain exact resolve enrichment once the same revision exact path is ready, head_item={head_item:?}, resolved_head_item={resolved_head_item:?}"
    );

    let exact_items = lsp_completion_items_with_request(
        &mut service,
        14003,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let exact_item = exact_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("exact completion item for Добавить");
    let exact_candidate_id = exact_item
        .data
        .as_ref()
        .and_then(|value| value.get("candidate_id"))
        .cloned()
        .expect("exact completion candidate_id");
    assert_eq!(
        exact_candidate_id, head_candidate_id,
        "same-revision head response and exact response must preserve stable candidate_id"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_resolve_stays_bound_to_origin_revision() {
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

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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

    let uri = Url::parse("file:///test_p33_completion_resolve_revision_binding.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_v2_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );
    server.sync_v2_globals().await;
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let head_items = lsp_completion_items_with_request(
        &mut service,
        14011,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let head_item = head_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("head completion item for Добавить");

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_v3_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );
    server.sync_v2_globals().await;

    let resolved_stale_item =
        lsp_completion_resolve_item_with_request(&mut service, 14012, head_item.clone()).await;
    assert_eq!(
        resolved_stale_item.detail, head_item.detail,
        "resolve must stay bound to the origin revision and fail closed once a newer revision supersedes the item"
    );
    assert_eq!(
        resolved_stale_item.documentation, head_item.documentation,
        "stale resolve must not enrich documentation from a newer revision"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_completion_head_upgrade_perf_report() {
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

    const FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const PROFILE_NAME: &str = "p33_completion_head_upgrade_perf_report";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );
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

    let uri = Url::parse("file:///test_p33_completion_exact_wait_recovery_perf.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");
    server.sync_v2_globals().await;
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let first_started = Instant::now();
    let first_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let first_elapsed_ms = first_started.elapsed().as_millis() as u64;
    assert!(
        first_completion_labels
            .iter()
            .any(|label| label == "Количество"),
        "first member-access completion must serve typed-structure members from current-revision head while matching exact precompute is still computing, labels={first_completion_labels:?}"
    );
    assert!(
        first_elapsed_ms < 250,
        "first head-path completion must stay bounded while exact precompute runs in background, elapsed_ms={first_elapsed_ms}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_started = Instant::now();
    let second_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let second_elapsed_ms = second_started.elapsed().as_millis() as u64;
    assert!(
        second_completion_labels
            .iter()
            .any(|label| label == "Количество"),
        "member-access completion must keep typed-structure members available after exact precompute finishes, labels={second_completion_labels:?}"
    );
    assert!(
        second_elapsed_ms < 250,
        "second completion must stay bounded after exact precompute finishes, elapsed_ms={second_elapsed_ms}"
    );

    let completion_timeline = lsp_get_completion_timeline(&mut service, 13321, 10).await;
    let observability_metrics = lsp_get_observability_metrics(&mut service, 13322).await;

    let timeline_traces = completion_timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let filtered_traces: Vec<serde_json::Value> = timeline_traces
        .iter()
        .filter(|trace| trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str()))
        .cloned()
        .collect();
    assert!(
        filtered_traces.len() >= 2,
        "expected at least two completion traces for perf report, filtered_traces={filtered_traces:?}"
    );
    let mut selected_traces: Vec<serde_json::Value> =
        filtered_traces.iter().rev().take(2).cloned().collect();
    selected_traces.reverse();
    let first_trace = &selected_traces[0];
    let second_trace = &selected_traces[1];

    let counters = observability_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = observability_metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let first_trace_id = first_trace
        .get("trace_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let first_outcome = first_trace
        .get("outcome")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let first_prepare_guard_outcome =
        completion_timeline_prepare_detail_str(first_trace, "guard_outcome").map(str::to_string);
    let first_prepare_outcome =
        completion_timeline_prepare_detail_str(first_trace, "outcome").map(str::to_string);
    let first_wait_exact_type_index_ms =
        completion_timeline_trace_stage_duration_ms(first_trace, "wait_exact_type_index")
            .unwrap_or(0);
    let first_prepare_stateful_ms =
        completion_timeline_trace_stage_duration_ms(first_trace, "prepare_stateful").unwrap_or(0);

    let second_trace_id = second_trace
        .get("trace_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let second_outcome = second_trace
        .get("outcome")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let second_prepare_guard_outcome =
        completion_timeline_prepare_detail_str(second_trace, "guard_outcome").map(str::to_string);
    let second_prepare_outcome =
        completion_timeline_prepare_detail_str(second_trace, "outcome").map(str::to_string);
    let second_wait_exact_type_index_ms =
        completion_timeline_trace_stage_duration_ms(second_trace, "wait_exact_type_index")
            .unwrap_or(0);
    let second_query_bundle_total_ms =
        completion_timeline_query_bundle_total_ms(second_trace).unwrap_or(0);
    let second_collect_ms =
        completion_timeline_trace_stage_duration_ms(second_trace, "collect").unwrap_or(0);

    let deadline_total = read_u64_metric(
        counters
            .get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
    );
    let ready_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
    );
    let no_matching_task_total = read_u64_metric(counters.get(
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task",
    ));
    let fail_closed_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));
    let ok_non_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
    let head_hit_total =
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit"));
    let exact_hit_total =
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit"));
    let head_to_exact_upgrade_total =
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total"));

    let selected_observability = serde_json::json!({
        "completion_duration_ms": histogram_metric_value(histograms, "completion_duration_ms", None),
        "intellisense_v2_wait_for_file_version_completion_ms": histogram_metric_value_or_zero(
            histograms,
            "intellisense_v2_wait_for_file_version_completion_ms",
            None
        ),
        "intellisense_v2_ir_query_completion_ms": histogram_metric_value_or_zero(
            histograms,
            "intellisense_v2_ir_query_completion_ms",
            None
        ),
        "completion_stage_prepare_stateful_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_prepare_stateful_ms",
            None
        ),
        "completion_stage_query_bundle_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_query_bundle_ms",
            None
        ),
        "completion_stage_collect_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_collect_ms",
            None
        ),
        "completion_stage_response_build_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_response_build_ms",
            None
        ),
        "intellisense_v2_completion_route_total_route_head_hit": head_hit_total,
        "intellisense_v2_completion_route_total_route_exact_hit": exact_hit_total,
        "intellisense_v2_completion_head_to_exact_upgrade_total": head_to_exact_upgrade_total,
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline": deadline_total,
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready": ready_total,
        "intellisense_v2_completion_fallback_unavailable_total": read_u64_metric(
            counters.get("intellisense_v2_completion_fallback_unavailable_total")
        ),
        "intellisense_v2_interactive_wait_budget_exhausted_total": read_u64_metric(
            counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
        ),
    });

    let report = serde_json::json!({
        "change_id": "refactor-v2-completion-dual-artifact-path",
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "fixture": {
            "uri": uri.as_str(),
            "file_id": file_id.0,
            "marker": "ДляCompletion = S.",
            "precompute_delay_ms": precompute_delay_ms,
            "wait_budget_ms": wait_budget_ms,
        },
        "requests": {
            "first": {
                "expected_behavior": "ok_non_empty_head_hit_while_exact_computes",
                "elapsed_ms": first_elapsed_ms,
                "label_count": first_completion_labels.len(),
                "labels": first_completion_labels,
            },
            "second": {
                "expected_behavior": "ok_non_empty_after_exact_upgrade",
                "elapsed_ms": second_elapsed_ms,
                "label_count": second_completion_labels.len(),
                "labels": second_completion_labels,
            }
        },
        "summary": {
            "first_trace_id": first_trace_id,
            "first_outcome": first_outcome,
            "first_prepare_guard_outcome": first_prepare_guard_outcome,
            "first_prepare_outcome": first_prepare_outcome,
            "first_wait_exact_type_index_ms": first_wait_exact_type_index_ms,
            "first_prepare_stateful_ms": first_prepare_stateful_ms,
            "second_trace_id": second_trace_id,
            "second_outcome": second_outcome,
            "second_prepare_guard_outcome": second_prepare_guard_outcome,
            "second_prepare_outcome": second_prepare_outcome,
            "second_wait_exact_type_index_ms": second_wait_exact_type_index_ms,
            "second_query_bundle_total_ms": second_query_bundle_total_ms,
            "second_collect_ms": second_collect_ms,
            "head_hit_total": head_hit_total,
            "exact_hit_total": exact_hit_total,
            "head_to_exact_upgrade_total": head_to_exact_upgrade_total,
            "deadline_total": deadline_total,
            "ready_total": ready_total,
            "no_matching_task_total": no_matching_task_total,
            "fail_closed_total": fail_closed_total,
            "ok_non_empty_total": ok_non_empty_total,
        },
        "completion_timeline": {
            "trace_count": filtered_traces.len(),
            "selected_traces": selected_traces,
            "raw": completion_timeline,
        },
        "observability": {
            "selected": selected_observability,
            "raw": observability_metrics,
        }
    });

    let report_path = std::env::var("BSL_V2_COMPLETION_DEADLINE_RECOVERY_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("completion-head-upgrade-perf.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p33 completion perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p33 completion perf report"),
    )
    .expect("write p33 completion perf report");
    println!("{}_path={}", PROFILE_NAME, report_path.display());

    assert_eq!(
        first_trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "first perf trace must capture ok_non_empty head response, trace={first_trace:?}"
    );
    assert_eq!(
        second_trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "second perf trace must capture post-upgrade completion outcome, trace={second_trace:?}"
    );
    assert_eq!(
        completion_timeline_prepare_detail_str(first_trace, "route"),
        Some("head_hit"),
        "first perf trace must capture head-hit route while exact precompute is still computing, trace={first_trace:?}"
    );
    assert!(
        matches!(
            completion_timeline_prepare_detail_str(second_trace, "route"),
            Some("head_hit" | "exact_hit")
        ),
        "second perf trace must stay on canonical head/exact route after upgrade, trace={second_trace:?}"
    );

    drain_task.abort();
}
