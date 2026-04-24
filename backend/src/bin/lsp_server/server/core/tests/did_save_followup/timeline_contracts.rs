#[tokio::test]
async fn p6_diagnostics_save_timeline_fastlane_fallback_bypasses_shared_queue_wait() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            Self::set_with_reload(key, value, false)
        }

        fn set_with_reload(key: &'static str, value: &str, reload_runtime_config: bool) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            if reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
            Self {
                key,
                previous,
                reload_runtime_config,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
            if self.reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест(\n    Возврат 1;\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "1500");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

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
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let uri =
        Url::parse("file:///did_save_fastlane_blocking_queue_wait_fixture.bsl").expect("fixture");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: V2_FIXTURE.to_string(),
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

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must materialize same-version ready parse snapshot");
    server
        .latest_ready_parse_snapshots_v2
        .write()
        .await
        .remove(&file_id);
    while published_rx.try_recv().is_ok() {}

    let blocker_count = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get().saturating_mul(4))
        .unwrap_or(16)
        .clamp(16, 128);
    let blocking_tasks = (0..blocker_count)
        .map(|_| {
            tokio::spawn(async move {
                let _ = bsl_runtime::application::spawn_bounded_blocking_with_class(
                    bsl_runtime::application::CpuWorkClass::Interactive,
                    move || {
                        std::thread::sleep(Duration::from_millis(350));
                    },
                )
                .await;
            })
        })
        .collect::<Vec<_>>();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let did_save_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification");
    assert!(did_save_response.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let Some(params) = published_rx.recv().await else {
                panic!("publish stream closed before fastlane publish");
            };
            if params.uri == uri && params.version == Some(2) && !params.diagnostics.is_empty() {
                break;
            }
        }
    })
    .await
    .expect("queued fastlane fallback must still publish");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_704, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching diagnostics save timeline trace");
    let first_publish = trace
        .get("first_publish")
        .and_then(|value| value.as_object())
        .expect("first publish trace");
    let blocking_queue_wait_ms = first_publish
        .get("blocking_queue_wait_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or_default();
    let first_publish_elapsed_ms = first_publish
        .get("elapsed_ms")
        .and_then(|value| value.as_u64())
        .unwrap_or(u64::MAX);
    assert!(
        blocking_queue_wait_ms == 0,
        "save_fastlane bypass must not inherit shared interactive queue wait, trace={trace:?}"
    );
    assert!(
        first_publish_elapsed_ms < 1_000,
        "save_fastlane first publish must stay bounded under shared queue saturation, trace={trace:?}"
    );
    assert_eq!(
        first_publish
            .get("profile")
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        first_publish
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("syntax_only")
    );
    assert!(
        first_publish.contains_key("syntax_diagnostics_query_ms"),
        "first publish must still expose syntax query timing field, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        Some(1),
        "first didSave cycle must expose dedicated monotonic save cycle sequence"
    );

    for task in blocking_tasks {
        let _ = task.await;
    }
    drain_task.abort();
}

#[tokio::test]
async fn p6_diagnostics_save_timeline_exposes_inflight_cycle_after_fastlane_before_idle_heavy_terminal(
) {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
        reload_runtime_config: bool,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            Self::set_with_reload(key, value, false)
        }

        fn set_with_reload(key: &'static str, value: &str, reload_runtime_config: bool) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            if reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
            Self {
                key,
                previous,
                reload_runtime_config,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
            if self.reload_runtime_config {
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }
    }

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест(\n    Возврат 1;\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "1500");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///did_save_timeline_inflight_cycle_fixture.bsl").expect("fixture");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: V2_FIXTURE.to_string(),
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
    while published_rx.try_recv().is_ok() {}

    let did_save_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification");
    assert!(did_save_response.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            let Some(params) = published_rx.recv().await else {
                panic!("publish stream closed before fastlane publish");
            };
            if params.uri == uri && params.version == Some(2) && !params.diagnostics.is_empty() {
                break;
            }
        }
    })
    .await
    .expect("save_fastlane must publish before delayed idle_heavy terminal");

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_702, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching inflight didSave diagnostics trace");

    assert_eq!(
        trace.get("trigger").and_then(|value| value.as_str()),
        Some("did_save")
    );
    assert_eq!(
        trace
            .get("save_fastlane_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert!(
        trace.get("idle_heavy_outcome").is_none(),
        "idle_heavy must stay pending while active didSave cycle is still in-flight, trace={trace:?}"
    );
    assert!(
        trace.get("terminal_outcome").is_none(),
        "in-flight save cycle must stay visible before terminal outcome, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("first_publish")
            .and_then(|value| value.get("profile"))
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        trace
            .get("first_publish")
            .and_then(|value| value.get("publish_kind"))
            .and_then(|value| value.as_str()),
        Some("syntax_only")
    );
    assert!(
        trace.get("followup_publish").is_none(),
        "follow-up publish must remain absent before idle_heavy completes, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_diagnostics_save_timeline_preserves_previous_cycle_when_next_did_save_supersedes_followup(
) {
    let coordinator = Arc::new(SystemCoordinator::new());
    let holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let holder = holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });
    initialize_lsp_service(&mut service).await;

    let server = holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let uri =
        Url::parse("file:///did_save_timeline_overlapping_cycles_fixture.bsl").expect("fixture");
    let first_key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(41),
        diagnostics_generation: 3,
        save_cycle_sequence: 1,
        requested_version: 2,
    };
    let second_key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: first_key.file_id,
        diagnostics_generation: 5,
        save_cycle_sequence: 2,
        requested_version: 3,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, first_key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        first_key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 15,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(7),
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

    server.begin_diagnostics_save_timeline_cycle(&uri, second_key);
    server.record_diagnostics_save_timeline_profile_disposition(
        &uri,
        first_key,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
    );
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        second_key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 11,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(5),
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

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_703, 12).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace_v2 = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("trace for first didSave cycle must be preserved");
    let trace_v3 = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(3)
        })
        .expect("trace for second didSave cycle must be preserved");

    assert_ne!(
        trace_v2.get("trace_id"),
        trace_v3.get("trace_id"),
        "sequential didSave cycles must keep distinct trace identities"
    );
    assert_eq!(
        trace_v2
            .get("first_publish")
            .and_then(|value| value.get("profile"))
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        trace_v2
            .get("save_fastlane_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        trace_v2
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("superseded_generation")
    );
    assert_eq!(
        trace_v2
            .get("terminal_outcome")
            .and_then(|value| value.as_str()),
        Some("superseded_generation")
    );
    assert_eq!(
        trace_v3
            .get("first_publish")
            .and_then(|value| value.get("profile"))
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        trace_v2
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        trace_v3
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert!(
        trace_v3
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
            > trace_v2
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64())
                .unwrap_or_default(),
        "new didSave cycle must keep a fresher dedicated save cycle sequence"
    );
    assert!(
        trace_v3.get("terminal_outcome").is_none(),
        "newer overlapping didSave cycle must remain independently visible while still active"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_diagnostics_save_timeline_same_requested_version_uses_save_cycle_sequence_for_correlation(
) {
    let coordinator = Arc::new(SystemCoordinator::new());
    let holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let holder = holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });
    initialize_lsp_service(&mut service).await;

    let server = holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let uri = Url::parse("file:///did_save_timeline_same_requested_version_fixture.bsl")
        .expect("fixture");
    let first_key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(42),
        diagnostics_generation: 13,
        save_cycle_sequence: 1,
        requested_version: 11,
    };
    let second_key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: first_key.file_id,
        diagnostics_generation: 12,
        save_cycle_sequence: 2,
        requested_version: 11,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, first_key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        first_key,
        crate::server::DiagnosticsSaveTimelineProfileResult {
            profile: bsl_runtime::application::DiagnosticsProfile::SaveFastlane,
            disposition: bsl_runtime::application::DiagnosticsDisposition::Published,
            publish: Some(crate::types::DiagnosticsSaveTimelinePublishTrace {
                profile: "save_fastlane".to_string(),
                publish_kind: "syntax_only".to_string(),
                outcome: "published".to_string(),
                elapsed_ms: 18,
                syntax_work_mode: Some("recomputed".to_string()),
                semantic_path: None,
                semantic_parse_source: None,
                semantic_ir_source: None,
                runtime_queue_wait_ms: None,
                apply_lag_ms: None,
                blocking_queue_wait_ms: None,
                wait_for_file_version_ms: None,
                snapshot_with_deps_ms: None,
                syntax_diagnostics_query_ms: Some(8),
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
    server.record_diagnostics_save_timeline_profile_disposition(
        &uri,
        first_key,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
    );

    server.begin_diagnostics_save_timeline_cycle(&uri, second_key);
    server.record_diagnostics_save_timeline_profile_result(
        &uri,
        second_key,
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
                syntax_diagnostics_query_ms: Some(5),
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

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_705, 12).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let matching = traces
        .iter()
        .filter(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(11)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        2,
        "same requested_version must still preserve two distinct didSave cycles"
    );

    let first_trace = matching
        .iter()
        .find(|trace| {
            trace
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64())
                == Some(1)
        })
        .expect("first save cycle sequence trace");
    let second_trace = matching
        .iter()
        .find(|trace| {
            trace
                .get("save_cycle_sequence")
                .and_then(|value| value.as_u64())
                == Some(2)
        })
        .expect("second save cycle sequence trace");

    assert_eq!(
        first_trace
            .get("diagnostics_generation")
            .and_then(|value| value.as_u64()),
        Some(13)
    );
    assert_eq!(
        second_trace
            .get("diagnostics_generation")
            .and_then(|value| value.as_u64()),
        Some(12)
    );
    assert_eq!(
        second_trace
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert!(
        second_trace
            .get("diagnostics_generation")
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
            < first_trace
                .get("diagnostics_generation")
                .and_then(|value| value.as_u64())
                .unwrap_or_default(),
        "test fixture must prove save ordering no longer depends on diagnostics generation"
    );

    drain_task.abort();
}

#[test]
fn p6_diagnostics_save_timeline_duration_to_nonzero_ms_filters_sub_ms_values() {
    assert_eq!(duration_to_nonzero_ms(None), None);
    assert_eq!(
        duration_to_nonzero_ms(Some(Duration::from_micros(999))),
        None
    );
    assert_eq!(
        duration_to_nonzero_ms(Some(Duration::from_millis(1))),
        Some(1)
    );
}

#[tokio::test]
async fn p6_diagnostics_save_timeline_followup_wait_state_ignores_sub_ms_runtime_facts() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let holder = holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });
    initialize_lsp_service(&mut service).await;

    let server = holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let uri =
        Url::parse("file:///did_save_timeline_sub_ms_runtime_facts_fixture.bsl").expect("fixture");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(43),
        diagnostics_generation: 17,
        save_cycle_sequence: 3,
        requested_version: 14,
    };

    server.begin_diagnostics_save_timeline_cycle(&uri, key);
    server.record_diagnostics_save_timeline_followup_wait_state(
        &uri,
        key,
        "runtime_queue_wait",
        Some(Duration::from_micros(500)),
        Some(Duration::from_micros(750)),
        Some(Duration::from_micros(900)),
        Some(Duration::from_micros(950)),
        Some("reused"),
        Some("generic_pipeline"),
        None,
        None,
        None,
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_706, 12).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(14)
        })
        .expect("sub-ms diagnostics save trace");

    assert_eq!(
        trace
            .get("followup_wait_reason")
            .and_then(|value| value.as_str()),
        Some("runtime_queue_wait")
    );
    assert!(
        trace.get("followup_runtime_queue_wait_ms").is_none(),
        "sub-ms runtime queue wait must be omitted instead of leaking as 0ms, trace={trace:?}"
    );
    assert!(
        trace.get("followup_apply_lag_ms").is_none(),
        "sub-ms apply lag must be omitted instead of leaking as 0ms, trace={trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_diagnostics_save_timeline_groups_fastlane_and_idle_heavy_under_one_save_cycle() {
    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест(\n    Возврат 1;\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///did_save_timeline_grouping_fixture.bsl").expect("fixture");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: V2_FIXTURE.to_string(),
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
    while published_rx.try_recv().is_ok() {}

    let did_save_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification");
    assert!(did_save_response.is_none(), "didSave is a notification");

    let mut version_two_publish_count = 0usize;
    let publish_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < publish_deadline && version_two_publish_count < 2 {
        let remaining = publish_deadline.saturating_duration_since(Instant::now());
        let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
            break;
        };
        let Some(params) = next else {
            break;
        };
        if params.uri == uri && params.version == Some(2) {
            version_two_publish_count += 1;
        }
    }
    assert!(
        version_two_publish_count >= 2,
        "expected save_fastlane and idle_heavy publishes for didSave cycle"
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_701, 8).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching didSave diagnostics trace");

    assert_eq!(
        trace.get("trigger").and_then(|value| value.as_str()),
        Some("did_save")
    );
    let diagnostics_generation = trace
        .get("diagnostics_generation")
        .and_then(|value| value.as_u64());
    assert!(
        diagnostics_generation.is_some_and(|value| value >= 2),
        "didSave trace must expose the save generation; same-version didChange diagnostics may be preempted by the save follow-up, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("save_cycle_sequence")
            .and_then(|value| value.as_u64()),
        Some(1),
        "first didSave for file must expose save cycle sequence independent from diagnostics generation"
    );
    assert_eq!(
        trace
            .get("first_publish")
            .and_then(|value| value.get("profile"))
            .and_then(|value| value.as_str()),
        Some("save_fastlane")
    );
    assert_eq!(
        trace
            .get("followup_publish")
            .and_then(|value| value.get("profile"))
            .and_then(|value| value.as_str()),
        Some("idle_heavy")
    );
    assert_eq!(
        trace
            .get("save_fastlane_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        trace
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        trace
            .get("terminal_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_type_index_precompute_slot_tracks_latest_version_and_clears_on_did_close() {
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

    let uri = Url::parse("file:///type-index-precompute-slot-v2.bsl").expect("test uri");
    let base_text =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: base_text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let latest_version = 8_i32;
    for version in 2..=latest_version {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: base_text.to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let observed_latest = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied();
    assert_eq!(
        observed_latest,
        Some(latest_version),
        "latest received version must track the newest didChange"
    );

    tokio::time::sleep(Duration::from_millis(30)).await;
    {
        let tasks = server.type_index_precompute_tasks_v2.lock().await;
        assert!(
            tasks.len() <= 1,
            "precompute scheduler must keep at most one task slot per file"
        );
        if let Some(task) = tasks.get(&file_id) {
            assert_eq!(
                task.supersession_key.requested_version, latest_version,
                "active precompute slot must target latest requested version"
            );
        }
    }

    let did_close = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    let did_close_req = Request::build("textDocument/didClose")
        .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
        .finish();
    let did_close_response = service
        .ready()
        .await
        .unwrap()
        .call(did_close_req)
        .await
        .expect("didClose notification");
    assert!(did_close_response.is_none(), "didClose is a notification");

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if !server
                .type_index_precompute_tasks_v2
                .lock()
                .await
                .contains_key(&file_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("type_index precompute slot must be cleared after didClose");

    drain_task.abort();
}

#[tokio::test]
async fn p6_did_close_records_client_cancel_for_inflight_diagnostics() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///did-close-cancel.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let did_close = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    let did_close_req = Request::build("textDocument/didClose")
        .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
        .finish();
    let did_close_response = service
        .ready()
        .await
        .unwrap()
        .call(did_close_req)
        .await
        .expect("didClose notification");
    assert!(did_close_response.is_none(), "didClose is a notification");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_client_cancel = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
            && key.contains("reason_client_cancel")
            && metric_number(value) > 0.0
    });
    assert!(
        saw_client_cancel,
        "didClose must record diagnostics pipeline client_cancel for removed in-flight tasks"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_idle_heavy_supersession_is_reported_for_burst_did_change() {
    struct DiagnosticsDebounceEnvGuard {
        previous_debounce_ms: Option<String>,
    }

    impl DiagnosticsDebounceEnvGuard {
        fn new() -> Self {
            Self {
                previous_debounce_ms: std::env::var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS").ok(),
            }
        }

        fn apply(&self, debounce_ms: u64) {
            std::env::set_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", debounce_ms.to_string());
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    impl Drop for DiagnosticsDebounceEnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous_debounce_ms {
                std::env::set_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", value);
            } else {
                std::env::remove_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS");
            }
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    let debounce_env_guard = DiagnosticsDebounceEnvGuard::new();
    debounce_env_guard.apply(500);

    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": true,
                    "showUnionDetails": true,
                    "minCertainty": 0.5
                },
                "codeActions": {
                    "enabled": false
                },
                "enableFlowSensitive": true
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_response = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_response.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///idle-heavy-supersession.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v2 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response_v2 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v2)
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_response_v2.is_none(),
        "didChange is a notification"
    );

    tokio::time::sleep(Duration::from_millis(20)).await;

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v3 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response_v3 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v3)
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_response_v3.is_none(),
        "didChange is a notification"
    );

    tokio::time::sleep(Duration::from_millis(800)).await;

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_idle_heavy_superseded = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_idle_profile_idle_heavy_reason_superseded_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        saw_idle_heavy_superseded,
        "burst didChange must produce superseded cancellation in idle_heavy profile"
    );

    drain_task.abort();
}
