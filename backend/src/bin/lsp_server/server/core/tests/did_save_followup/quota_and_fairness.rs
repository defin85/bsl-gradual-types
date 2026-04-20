#[tokio::test]
async fn p32_ranged_did_change_program_lowering_retarget_preserves_parser_base_for_newer_target() {
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..256 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    fn utf16_range_for_substring(source: &str, needle: &str) -> Range {
        let start_byte = source
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"));
        let end_byte = start_byte + needle.len();
        let start = &source[..start_byte];
        let end = &source[..end_byte];
        let start_line = start.lines().count().saturating_sub(1) as u32;
        let start_character = start
            .lines()
            .last()
            .unwrap_or("")
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;
        let end_line = end.lines().count().saturating_sub(1) as u32;
        let end_character = end
            .lines()
            .last()
            .unwrap_or("")
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;
        Range {
            start: Position::new(start_line, start_character),
            end: Position::new(end_line, end_character),
        }
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard = EnvVarGuard::set("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0");
    let _conversion_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS",
        "40",
    );

    let v1_fixture = build_fixture("v1");
    let v2_fixture = v1_fixture.replacen("v1-128", "v2-128", 1);
    let v3_fixture = v2_fixture.replacen("v2-128", "v3-128", 1);
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_ranged_program_lowering_parser_base_fixture.bsl",
    )
    .await;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let ready = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .cloned();
            if ready
                .as_ref()
                .is_some_and(|state| state.parse_snapshot.file_version == 1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(
        "opened fixture must materialize version 1 before ranged parser-base preservation test",
    );

    let baseline_metrics = server.coordinator.observability_metrics();
    let baseline_counters = baseline_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("baseline metrics.counters object");

    let did_change_v2_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_substring(&v1_fixture, "v1-128")),
                            range_length: None,
                            text: "v2-128".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let current_checkpoint = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_assembly_checkpoint
                });
            if current_checkpoint
                == Some(crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ranged v2 worker must enter program lowering before retarget");

    tokio::time::sleep(Duration::from_millis(150)).await;
    let sustained_checkpoint = server
        .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
        .await
        .and_then(|control| {
            control
                .phase_attribution_snapshot()
                .current_assembly_checkpoint
        });
    assert_eq!(
        sustained_checkpoint,
        Some(crate::server::ReadyParseSnapshotAssemblyCheckpointV2::ProgramLowering),
        "ranged local-edit fixture must keep the worker inside program lowering long enough to prove reused-lowering checkpoints"
    );

    let did_change_v3_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: 3,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: Some(utf16_range_for_substring(&v2_fixture, "v2-128")),
                            range_length: None,
                            text: "v3-128".to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(3) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("newer ranged target must materialize after late parser-base preservation");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    assert_eq!(
        retargeted_during_parse_delta, 0,
        "late ranged preservation should stop cancelling at parse stage, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert!(
        retargeted_before_materialization_delta > 0,
        "late ranged preservation should hand off through retargeted_before_materialization instead, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let evidence = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let execute = Request::build("workspace/executeCommand")
                .id(32003)
                .params(serde_json::json!({
                    "command": "bsl.getObservabilityMetrics",
                    "arguments": [],
                }))
                .finish();
            let execute_response = service
                .ready()
                .await
                .unwrap()
                .call(execute)
                .await
                .expect("workspace/executeCommand request")
                .expect("workspace/executeCommand response");
            let value = serde_json::to_value(&execute_response).expect("serialize response");
            let result = value.get("result").cloned().expect("result field");
            let evidence = result
                .get("didChangeParseSnapshotEvidence")
                .and_then(|value| value.get("entries"))
                .and_then(|value| value.as_array())
                .and_then(|entries| {
                    entries.iter().find(|entry| {
                        entry.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && entry
                                .get("requestedVersion")
                                .and_then(|value| value.as_i64())
                                == Some(3)
                    })
                })
                .cloned();
            if let Some(entry) = evidence {
                break entry;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("ranged v3 didChange evidence must appear in observability metrics");

    assert_eq!(
        evidence.get("parseMode").and_then(|value| value.as_str()),
        Some("incremental")
    );
    assert_eq!(
        evidence
            .get("fallbackReason")
            .and_then(|value| value.as_str()),
        None,
        "late ranged parser-base preservation must avoid stale_parser_base fallback"
    );
    assert_eq!(
        evidence
            .get("parserBaseRootCause")
            .and_then(|value| value.as_str()),
        None
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after ranged parser-base preservation");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p8_did_save_followup_quota_zero_disables_future_admissions_without_revoking_admitted_work()
{
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
    const V2_FIXTURE_A: &str = "Процедура Тест()\n    Сообщить(необъявленнаяА);\nКонецПроцедуры\n";
    const V2_FIXTURE_B: &str = "Процедура Тест()\n    Сообщить(необъявленнаяБ);\nКонецПроцедуры\n";
    const FOLLOWUP_PUBLISH_BUDGET_MS: u64 = 10_000;

    let _env_lock = lock_test_env().await;
    let _background_reserved_only_guard =
        EnvVarGuard::set("BSL_TEST_RUNTIME_BACKGROUND_RESERVED_ONLY", "1");
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);
    let _quota_guard = EnvVarGuard::set_with_reload(
        "BSL_INTELLISENSE_V2_DID_SAVE_FOLLOWUP_LANE_QUOTA",
        "1",
        true,
    );

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
    let uri_a = Url::parse("file:///did_save_followup_quota_zero_a_fixture.bsl").expect("uri a");
    let uri_b = Url::parse("file:///did_save_followup_quota_zero_b_fixture.bsl").expect("uri b");
    for uri in [&uri_a, &uri_b] {
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didOpen")
                    .params(
                        serde_json::to_value(DidOpenTextDocumentParams {
                            text_document: TextDocumentItem {
                                uri: uri.clone(),
                                language_id: "bsl".to_string(),
                                version: 1,
                                text: V1_FIXTURE.to_string(),
                            },
                        })
                        .expect("DidOpenTextDocumentParams"),
                    )
                    .finish(),
            )
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
    }

    for (uri, text) in [(&uri_a, V2_FIXTURE_A), (&uri_b, V2_FIXTURE_B)] {
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didChange")
                    .params(
                        serde_json::to_value(DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: uri.clone(),
                                version: 2,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: text.to_string(),
                            }],
                        })
                        .expect("DidChangeTextDocumentParams"),
                    )
                    .finish(),
            )
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }
    while published_rx.try_recv().is_ok() {}

    let background_holder_barrier = Arc::new(std::sync::Barrier::new(2));
    let (background_holder_ready_tx, background_holder_ready_rx) = tokio::sync::oneshot::channel();
    let background_holder_barrier_for_task = background_holder_barrier.clone();
    let background_holder_coordinator = coordinator.clone();
    let background_holder = tokio::spawn(async move {
        let _ = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::CpuWorkClass::Background,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(background_holder_coordinator.as_ref()),
            move || {
                let _ = background_holder_ready_tx.send(());
                background_holder_barrier_for_task.wait();
            },
        )
        .await;
    });
    tokio::time::timeout(Duration::from_secs(3), background_holder_ready_rx)
        .await
        .expect("background holder must start before didSave follow-up")
        .expect("background holder ready signal");

    let did_save_a_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_a.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification for file A");
    assert!(did_save_a_response.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri == uri_a && params.version == Some(2) {
                break params;
            }
        }
    })
    .await
    .expect("save_fastlane first publish for file A must arrive");

    let did_save_b_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_b.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave notification for file B");
    assert!(did_save_b_response.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri == uri_b && params.version == Some(2) {
                break params;
            }
        }
    })
    .await
    .expect("save_fastlane first publish for file B must arrive");

    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_714, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            if traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_b.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("followup_wait_reason")
                        .and_then(|value| value.as_str())
                        == Some("runtime_queue_wait")
                    && trace.get("followup_publish").is_none()
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("file B follow-up must queue behind the admitted dedicated lane slot");

    let _quota_disable_guard = EnvVarGuard::set_with_reload(
        "BSL_INTELLISENSE_V2_DID_SAVE_FOLLOWUP_LANE_QUOTA",
        "0",
        true,
    );
    background_holder_barrier.wait();
    tokio::time::timeout(Duration::from_secs(3), background_holder)
        .await
        .expect("background holder task timeout")
        .expect("background holder join");

    tokio::time::timeout(Duration::from_millis(FOLLOWUP_PUBLISH_BUDGET_MS), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri != uri_a || params.version != Some(2) {
                continue;
            }
            if params
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
            {
                break params;
            }
        }
    })
    .await
    .expect("already admitted file A follow-up must still publish full diagnostics");

    let timeline = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_715, 16).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            let file_a_ready = traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_a.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("idle_heavy_outcome")
                        .and_then(|value| value.as_str())
                        == Some("published")
            });
            let file_b_ready = traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_b.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("idle_heavy_outcome")
                        .and_then(|value| value.as_str())
                        == Some("disabled_by_config")
                    && trace
                        .get("terminal_outcome")
                        .and_then(|value| value.as_str())
                        == Some("disabled_by_config")
            });
            if file_a_ready && file_b_ready {
                break timeline;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("timeline must reflect admitted-vs-queued quota-zero outcomes");
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    let trace_a = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri_a.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching file A diagnostics save timeline trace");
    let trace_b = traces
        .iter()
        .find(|trace| {
            trace.get("uri").and_then(|value| value.as_str()) == Some(uri_b.as_str())
                && trace
                    .get("requested_version")
                    .and_then(|value| value.as_i64())
                    == Some(2)
        })
        .expect("matching file B diagnostics save timeline trace");
    assert_eq!(
        trace_a
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("published"),
        "already admitted file A follow-up must stay published after later quota disable, trace={trace_a:?}"
    );
    assert!(
        trace_a.get("followup_publish").is_some(),
        "already admitted file A follow-up must still emit a full publish trace, trace={trace_a:?}"
    );
    assert_eq!(
        trace_b
            .get("save_fastlane_outcome")
            .and_then(|value| value.as_str()),
        Some("published"),
        "quota=0 must not affect file B save_fastlane first publish, trace={trace_b:?}"
    );
    assert_eq!(
        trace_b
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("disabled_by_config"),
        "queued file B follow-up must re-check quota at admission and terminate explicitly, trace={trace_b:?}"
    );
    assert_eq!(
        trace_b
            .get("terminal_outcome")
            .and_then(|value| value.as_str()),
        Some("disabled_by_config"),
        "file B terminal outcome must preserve disabled_by_config canonically, trace={trace_b:?}"
    );
    let trace_b_followup_publish = trace_b
        .get("followup_publish")
        .and_then(|value| value.as_object())
        .expect("disabled file B follow-up must still emit canonical terminal trace");
    assert_eq!(
        trace_b_followup_publish
            .get("outcome")
            .and_then(|value| value.as_str()),
        Some("disabled_by_config"),
        "disabled file B follow-up trace must preserve disabled_by_config canonically, trace={trace_b:?}"
    );
    assert_eq!(
        trace_b_followup_publish
            .get("publish_kind")
            .and_then(|value| value.as_str()),
        Some("unknown"),
        "disabled file B follow-up trace must stay non-publish-shaped even when represented terminally, trace={trace_b:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        counters
            .get("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_save_profile_idle_heavy_reason_disabled_by_config")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "quota=0 follow-up disable must flow through shared diagnostics pipeline outcome counters, metrics={metrics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p8_did_save_followup_default_quota_keeps_single_slot_latest_only_cross_file_fairness() {
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
    const V2_FIXTURE_A: &str = "Процедура Тест()\n    Сообщить(необъявленнаяА2);\nКонецПроцедуры\n";
    const V3_FIXTURE_A: &str = "Процедура Тест()\n    Сообщить(необъявленнаяА3);\nКонецПроцедуры\n";
    const V4_FIXTURE_A: &str = "Процедура Тест()\n    Сообщить(необъявленнаяА4);\nКонецПроцедуры\n";
    const V2_FIXTURE_B: &str = "Процедура Тест()\n    Сообщить(необъявленнаяБ2);\nКонецПроцедуры\n";

    let _env_lock = lock_test_env().await;
    let _background_reserved_only_guard =
        EnvVarGuard::set("BSL_TEST_RUNTIME_BACKGROUND_RESERVED_ONLY", "1");
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
    let uri_a = Url::parse("file:///did_save_followup_default_quota_fairness_a_fixture.bsl")
        .expect("uri a");
    let uri_b = Url::parse("file:///did_save_followup_default_quota_fairness_b_fixture.bsl")
        .expect("uri b");
    for uri in [&uri_a, &uri_b] {
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didOpen")
                    .params(
                        serde_json::to_value(DidOpenTextDocumentParams {
                            text_document: TextDocumentItem {
                                uri: uri.clone(),
                                language_id: "bsl".to_string(),
                                version: 1,
                                text: V1_FIXTURE.to_string(),
                            },
                        })
                        .expect("DidOpenTextDocumentParams"),
                    )
                    .finish(),
            )
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
    }

    let did_change_a_v2 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri_a.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE_A.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange A v2 notification");
    assert!(did_change_a_v2.is_none(), "didChange is a notification");
    let did_change_b_v2 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri_b.clone(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE_B.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange B v2 notification");
    assert!(did_change_b_v2.is_none(), "didChange is a notification");
    while published_rx.try_recv().is_ok() {}

    let background_holder_barrier = Arc::new(std::sync::Barrier::new(2));
    let (background_holder_ready_tx, background_holder_ready_rx) = tokio::sync::oneshot::channel();
    let background_holder_barrier_for_task = background_holder_barrier.clone();
    let background_holder_coordinator = coordinator.clone();
    let background_holder = tokio::spawn(async move {
        let _ = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::CpuWorkClass::Background,
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            Some(background_holder_coordinator.as_ref()),
            move || {
                let _ = background_holder_ready_tx.send(());
                background_holder_barrier_for_task.wait();
            },
        )
        .await;
    });
    tokio::time::timeout(Duration::from_secs(3), background_holder_ready_rx)
        .await
        .expect("background holder must start before didSave follow-up")
        .expect("background holder ready signal");

    let did_save_a_v2 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_a.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave A v2 notification");
    assert!(did_save_a_v2.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri == uri_a && params.version == Some(2) {
                break params;
            }
        }
    })
    .await
    .expect("save_fastlane first publish for A v2 must arrive");

    let did_save_b_v2 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_b.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave B v2 notification");
    assert!(did_save_b_v2.is_none(), "didSave is a notification");

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri == uri_b && params.version == Some(2) {
                break params;
            }
        }
    })
    .await
    .expect("save_fastlane first publish for B v2 must arrive");

    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_716, 24).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            if traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_b.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("followup_wait_reason")
                        .and_then(|value| value.as_str())
                        == Some("runtime_queue_wait")
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("default quota must leave B queued behind the single admitted A follow-up");

    let did_change_a_v3 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri_a.clone(),
                            version: 3,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: V3_FIXTURE_A.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange A v3 notification");
    assert!(did_change_a_v3.is_none(), "didChange is a notification");
    let did_save_a_v3 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_a.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave A v3 notification");
    assert!(did_save_a_v3.is_none(), "didSave is a notification");

    let did_change_a_v4 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(
                    serde_json::to_value(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri_a.clone(),
                            version: 4,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: V4_FIXTURE_A.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didChange A v4 notification");
    assert!(did_change_a_v4.is_none(), "didChange is a notification");
    let did_save_a_v4 = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didSave")
                .params(
                    serde_json::to_value(DidSaveTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri_a.clone() },
                        text: None,
                    })
                    .expect("DidSaveTextDocumentParams"),
                )
                .finish(),
        )
        .await
        .expect("didSave A v4 notification");
    assert!(did_save_a_v4.is_none(), "didSave is a notification");

    background_holder_barrier.wait();
    tokio::time::timeout(Duration::from_secs(3), background_holder)
        .await
        .expect("background holder task timeout")
        .expect("background holder join");

    let publish_order = tokio::time::timeout(Duration::from_secs(10), async {
        let mut order = Vec::new();
        let mut saw_b_v2 = false;
        let mut saw_a_v4 = false;
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if !params
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
            {
                continue;
            }
            let version = params.version.unwrap_or_default();
            if params.uri == uri_b && version == 2 && !saw_b_v2 {
                saw_b_v2 = true;
                order.push(("b", version));
            } else if params.uri == uri_a && version == 4 && !saw_a_v4 {
                saw_a_v4 = true;
                order.push(("a", version));
            }
            if saw_b_v2 && saw_a_v4 {
                break order;
            }
        }
    })
    .await
    .expect("queued B v2 and latest A v4 full publishes must both complete");
    assert_eq!(
        publish_order,
        vec![("b", 2), ("a", 4)],
        "default single-slot dedicated lane must let queued B v2 run before noisy-file A v4 and must shed stale same-file work instead of raw FIFO blocking, order={publish_order:?}"
    );

    let timeline = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_717, 24).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            let has_b_v2 = traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_b.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && trace
                        .get("idle_heavy_outcome")
                        .and_then(|value| value.as_str())
                        == Some("published")
            });
            let has_a_v4 = traces.iter().any(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri_a.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(4)
                    && trace
                        .get("idle_heavy_outcome")
                        .and_then(|value| value.as_str())
                        == Some("published")
            });
            if has_b_v2 && has_a_v4 {
                break timeline;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("timeline must show B v2 published and latest A v4 published");
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("diagnostics save timeline traces");
    if let Some(trace_a_v3) = traces.iter().find(|trace| {
        trace.get("uri").and_then(|value| value.as_str()) == Some(uri_a.as_str())
            && trace
                .get("requested_version")
                .and_then(|value| value.as_i64())
                == Some(3)
    }) {
        let followup_outcome = trace_a_v3
            .get("followup_publish")
            .and_then(|value| value.get("outcome"))
            .and_then(|value| value.as_str());
        assert_ne!(
            followup_outcome,
            Some("published"),
            "older queued A v3 follow-up must not survive as a full publish once A v4 supersedes it, trace={trace_a_v3:?}"
        );
    }

    let metrics = coordinator.observability_metrics();
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    assert_eq!(
        gauges
            .get("intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_quota")
            .and_then(|value| value.as_f64()),
        Some(1.0),
        "default dedicated lane quota must stay operator-visible as 1 without overrides, metrics={metrics:?}"
    );

    drain_task.abort();
}
