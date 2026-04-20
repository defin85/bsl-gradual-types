#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_parse_delay_does_not_delay_concurrent_completion_live_transport() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 1_500;
    const CURRENT_CONTEXT_REQUEST_ID: i64 = 50_533;
    const COMPLETION_REQUEST_ID: i64 = 50_534;
    const RUNTIME_ISOLATION_BUDGET_MS: u64 = 250;

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );

    let mut fixture = String::new();
    for index in 0..256 {
        fixture.push_str(&format!(
            "Процедура Вспомогательная{}()\n    Сообщить(\"{}\");\nКонецПроцедуры\n\n",
            index, index
        ));
    }
    fixture.push_str(
        "Процедура ТестКонтекста(ПараметрКонтекста)\n    ДляCompletion = Объект.\n    Сообщить(ПараметрКонтекста);\nКонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_runtime_isolation_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.clone(),
            },
        })
        .await;
    server.sync_v2_globals().await;

    let completion_position = find_utf16_position_after_marker(&fixture, "ДляCompletion = Объект.");
    let current_context_position =
        find_utf16_position_after_marker(&fixture, "Сообщить(ПараметрКонтекста");

    live_transport_write_execute_command_request(
        &mut harness,
        CURRENT_CONTEXT_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": current_context_position.line,
            "character": current_context_position.character,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_request_written_at_ms = live_transport_write_completion_request(
        &mut harness,
        COMPLETION_REQUEST_ID,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    let (completion_response, completion_elapsed_ms, current_context_response) =
        tokio::time::timeout(Duration::from_secs(10), async {
            let completion_started = Instant::now();
            let mut completion_response = None;
            let mut completion_elapsed_ms = None;
            let mut current_context_response = None;
            loop {
                let response = harness.read_message().await;
                let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                    continue;
                };
                if response_id == COMPLETION_REQUEST_ID {
                    completion_elapsed_ms = Some(
                        completion_started
                            .elapsed()
                            .as_millis()
                            .min(u64::MAX as u128) as u64,
                    );
                    completion_response = Some(response);
                } else if response_id == CURRENT_CONTEXT_REQUEST_ID {
                    current_context_response = Some(response);
                }
                if completion_response.is_some() && current_context_response.is_some() {
                    break (
                        completion_response.take().expect("completion response"),
                        completion_elapsed_ms.expect("completion elapsed"),
                        current_context_response
                            .take()
                            .expect("current context response"),
                    );
                }
            }
        })
        .await
        .expect("completion and current-context responses must arrive");

    assert!(
        completion_response.get("result").is_some(),
        "completion must still return a response while getCurrentContext parse is delayed"
    );
    assert!(
        completion_elapsed_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "concurrent getCurrentContext parse delay must not stall completion end-to-end latency, completion_elapsed_ms={}ms, completion_response={completion_response:?}",
        completion_elapsed_ms
    );
    assert_eq!(
        current_context_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ТестКонтекста"),
        "getCurrentContext must preserve user-visible routine resolution after runtime isolation"
    );

    let trace = wait_for_live_completion_timeline_trace_with_server_edge_fields(
        &mut harness,
        50_535,
        32,
        COMPLETION_REQUEST_ID,
        &[
            "adapter_read_at_ms",
            "service_future_to_first_poll_wait_ms",
            "response_output_handoff_send_wait_ms",
        ],
    )
    .await;

    let client_to_transport_wait_ms =
        completion_timeline_server_edge_u64(&trace, "adapter_read_at_ms")
            .expect("adapter_read_at_ms")
            .saturating_sub(completion_request_written_at_ms);
    let service_future_to_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&trace, "service_future_to_first_poll_wait_ms")
            .expect("service_future_to_first_poll_wait_ms");
    let response_output_handoff_send_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_send_wait_ms")
            .expect("response_output_handoff_send_wait_ms");
    assert!(
        client_to_transport_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "concurrent getCurrentContext parse delay must not cause ingress starvation before adapter_read, client_to_transport_wait_ms={}ms, trace={trace:?}",
        client_to_transport_wait_ms
    );
    assert!(
        service_future_to_first_poll_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "concurrent getCurrentContext parse delay must not delay completion first poll, trace={trace:?}"
    );
    assert!(
        response_output_handoff_send_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "concurrent getCurrentContext parse delay must not strand completion response on output handoff, trace={trace:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_same_revision_burst_shares_one_broker_leader_before_blocking() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 900;
    const PARSE_GAP_DELAY_MS: u64 = 1_500;
    const FIRST_REQUEST_ID: i64 = 50_536;
    const SECOND_REQUEST_ID: i64 = 50_537;

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );
    let _parse_gap_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_GAP_DELAY_MS.to_string(),
    );

    let fixture = concat!(
        "Процедура ПерваяПроцедура(Первый)\n",
        "    Сообщить(Первый);\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура ВтораяПроцедура(Второй)\n",
        "    Сообщить(Второй);\n",
        "КонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_broker_burst_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.to_string(),
            },
        })
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let mut current_text = fixture.to_string();
    live_transport_append_text_change(&mut harness, &uri, &current_text, 2, "\n// broker-gap\n")
        .await;
    current_text.push_str("\n// broker-gap\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe didChange parse gap before broker burst");

    let first_position = find_utf16_position_after_marker(fixture, "Сообщить(Первый");
    let second_position = find_utf16_position_after_marker(fixture, "Сообщить(Второй");
    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": first_position.line,
            "character": first_position.character,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": second_position.line,
            "character": second_position.character,
        })],
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if response_id == FIRST_REQUEST_ID {
                first_response = Some(response);
            } else if response_id == SECOND_REQUEST_ID {
                second_response = Some(response);
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("burst current-context responses must arrive");

    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ПерваяПроцедура"),
        "first same-revision request must resolve against the shared leader parse"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ВтораяПроцедура"),
        "second same-revision request must reuse the shared leader parse for its own cursor position"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test(),
        1,
        "same-revision current-context burst must launch exactly one blocking parse leader"
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
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
        ),
        1,
        "burst must record exactly one broker leader role"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
        ),
        1,
        "burst must record exactly one broker follower role"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
        ),
        2,
        "both burst requests must resolve through the explicit broker contract"
    );
    let source_wall_histogram = histogram_metric_value_or_zero(
        histograms,
        "intellisense_v2_current_context_wall_ms_source_parser_coordinator",
        None,
    );
    assert!(
        source_wall_histogram
            .get("count")
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
            > 0,
        "brokered parser-coordinator path must record source-scoped wall latency, histogram={source_wall_histogram:?}"
    );
    let report = serde_json::json!({
        "change_id": "refactor-11-current-context-parse-broker-bounding",
        "profile": "current_context_broker_same_revision_burst_smoke",
        "command": "cargo test -p bsl-backend --bin bsl-lsp-server same_revision_burst_shares_one_broker_leader_before_blocking",
        "summary": {
            "parse_attempts": super::super::command_handlers::get_current_context_parse_attempts_for_test(),
            "broker_leader_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
            ),
            "broker_follower_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
            ),
            "resolved_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
            ),
            "budget_exhausted_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_terminal_total_outcome_budget_exhausted")
            ),
            "superseded_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_terminal_total_outcome_superseded")
            ),
            "parser_coordinator_source_total": read_u64_metric(
                counters.get("intellisense_v2_current_context_parse_source_total_source_parser_coordinator")
            ),
        },
        "selected_histograms": {
            "current_context_parse_ms_role_broker_leader": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_parse_ms_role_broker_leader",
                None
            ),
            "current_context_parse_ms_role_broker_follower": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_parse_ms_role_broker_follower",
                None
            ),
            "current_context_wall_ms_role_broker_leader": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_wall_ms_role_broker_leader",
                None
            ),
            "current_context_wall_ms_role_broker_follower": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_wall_ms_role_broker_follower",
                None
            ),
            "current_context_wall_ms_source_parser_coordinator": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_current_context_wall_ms_source_parser_coordinator",
                None
            ),
        },
    });
    let report_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("perf")
        .join("reports")
        .join("refactor-11-current-context-parse-broker-bounding-burst-smoke.json");
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent).expect("create current-context broker smoke report dir");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize current-context broker report"),
    )
    .expect("write current-context broker report");

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_broker_follower_budget_exhaustion_returns_bounded_empty() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 900;
    const PARSE_GAP_DELAY_MS: u64 = 1_500;
    const FOLLOWER_WAIT_BUDGET_MS: u64 = 150;
    const FIRST_REQUEST_ID: i64 = 50_538;
    const SECOND_REQUEST_ID: i64 = 50_539;

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );
    let _parse_gap_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_GAP_DELAY_MS.to_string(),
    );
    let _follower_wait_budget_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_BROKER_WAIT_BUDGET_MS",
        &FOLLOWER_WAIT_BUDGET_MS.to_string(),
    );

    let fixture = concat!(
        "Процедура ТестоваяПроцедура(Параметр)\n",
        "    Сообщить(Параметр);\n",
        "КонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_broker_budget_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.to_string(),
            },
        })
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let mut current_text = fixture.to_string();
    live_transport_append_text_change(&mut harness, &uri, &current_text, 2, "\n// broker-gap\n")
        .await;
    current_text.push_str("\n// broker-gap\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe didChange parse gap before budget-exhausted follower");

    let position = find_utf16_position_after_marker(fixture, "Сообщить(Параметр");
    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
        })],
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if response_id == FIRST_REQUEST_ID {
                first_response = Some(response);
            } else if response_id == SECOND_REQUEST_ID {
                second_response = Some(response);
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("leader and follower budget-exhaustion responses must arrive");

    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ТестоваяПроцедура"),
        "leader request must still resolve after completing the shared parse"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        None,
        "over-budget follower must fail closed with an empty current-context response"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionKind"))
            .and_then(|value| value.as_str()),
        Some("none"),
        "over-budget follower must degrade to the empty surface contract"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test(),
        1,
        "budget-exhausted follower must not launch an independent blocking parse holder"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
        ),
        1,
        "budget-exhaustion scenario must still produce one broker leader"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
        ),
        1,
        "budget-exhaustion scenario must still produce one broker follower"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_budget_exhausted")
        ),
        1,
        "over-budget follower must be visible in observability"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_same_key_current_context_burst_keeps_completion_bounded_under_mixed_load() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 1_500;
    const PARSE_GAP_DELAY_MS: u64 = 1_500;
    const FIRST_REQUEST_ID: i64 = 50_543;
    const SECOND_REQUEST_ID: i64 = 50_544;
    const COMPLETION_REQUEST_ID: i64 = 50_545;
    const RUNTIME_ISOLATION_BUDGET_MS: u64 = 250;

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );
    let _parse_gap_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_GAP_DELAY_MS.to_string(),
    );

    let mut fixture = String::new();
    for index in 0..256 {
        fixture.push_str(&format!(
            "Процедура Вспомогательная{}()\n    Сообщить(\"{}\");\nКонецПроцедуры\n\n",
            index, index
        ));
    }
    fixture.push_str(
        "Процедура ТестКонтекста(ПараметрКонтекста)\n    ДляCompletion = Объект.\n    Сообщить(ПараметрКонтекста);\nКонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_broker_mixed_load_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.clone(),
            },
        })
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let mut current_text = fixture.clone();
    live_transport_append_text_change(&mut harness, &uri, &current_text, 2, "\n// broker-gap\n")
        .await;
    current_text.push_str("\n// broker-gap\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe didChange parse gap before same-key mixed load");

    let completion_position = find_utf16_position_after_marker(&fixture, "ДляCompletion = Объект.");
    let current_context_position =
        find_utf16_position_after_marker(&fixture, "Сообщить(ПараметрКонтекста");
    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": current_context_position.line,
            "character": current_context_position.character,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": current_context_position.line,
            "character": current_context_position.character,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_request_written_at_ms = live_transport_write_completion_request(
        &mut harness,
        COMPLETION_REQUEST_ID,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    let (completion_response, completion_elapsed_ms, first_response, second_response) =
        tokio::time::timeout(Duration::from_secs(10), async {
            let completion_started = Instant::now();
            let mut completion_response = None;
            let mut completion_elapsed_ms = None;
            let mut first_response = None;
            let mut second_response = None;
            loop {
                let response = harness.read_message().await;
                let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                    continue;
                };
                if response_id == COMPLETION_REQUEST_ID {
                    completion_elapsed_ms = Some(
                        completion_started
                            .elapsed()
                            .as_millis()
                            .min(u64::MAX as u128) as u64,
                    );
                    completion_response = Some(response);
                } else if response_id == FIRST_REQUEST_ID {
                    first_response = Some(response);
                } else if response_id == SECOND_REQUEST_ID {
                    second_response = Some(response);
                }
                if completion_response.is_some()
                    && first_response.is_some()
                    && second_response.is_some()
                {
                    break (
                        completion_response.take().expect("completion response"),
                        completion_elapsed_ms.expect("completion elapsed"),
                        first_response.take().expect("first response"),
                        second_response.take().expect("second response"),
                    );
                }
            }
        })
        .await
        .expect("mixed-load completion and brokered current-context responses must arrive");

    assert!(
        completion_response.get("result").is_some(),
        "completion must still return a response while same-key current-context broker leader is delayed"
    );
    assert!(
        completion_elapsed_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "same-key current-context burst must not add extra blocking contention for completion, completion_elapsed_ms={}ms, completion_response={completion_response:?}",
        completion_elapsed_ms
    );
    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ТестКонтекста"),
        "leader current-context request must still resolve user-visible routine information"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ТестКонтекста"),
        "same-key follower request must reuse the shared leader parse instead of spawning a second blocking parse"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test(),
        1,
        "mixed-load same-key burst must still keep parse fan-out bounded to one leader"
    );

    let trace = wait_for_live_completion_timeline_trace_with_server_edge_fields(
        &mut harness,
        50_546,
        32,
        COMPLETION_REQUEST_ID,
        &[
            "adapter_read_at_ms",
            "service_future_to_first_poll_wait_ms",
            "response_output_handoff_send_wait_ms",
        ],
    )
    .await;

    let client_to_transport_wait_ms =
        completion_timeline_server_edge_u64(&trace, "adapter_read_at_ms")
            .expect("adapter_read_at_ms")
            .saturating_sub(completion_request_written_at_ms);
    let service_future_to_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&trace, "service_future_to_first_poll_wait_ms")
            .expect("service_future_to_first_poll_wait_ms");
    let response_output_handoff_send_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_send_wait_ms")
            .expect("response_output_handoff_send_wait_ms");
    assert!(
        client_to_transport_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "same-key current-context burst must not regress completion ingress before adapter_read, client_to_transport_wait_ms={}ms, trace={trace:?}",
        client_to_transport_wait_ms
    );
    assert!(
        service_future_to_first_poll_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "same-key current-context burst must not delay completion first poll, trace={trace:?}"
    );
    assert!(
        response_output_handoff_send_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "same-key current-context burst must not strand completion response on output handoff, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
        ),
        1,
        "mixed-load same-key burst must still record exactly one broker leader"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_follower")
        ),
        1,
        "mixed-load same-key burst must still record exactly one broker follower"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_inflight_non_equivalent_generation_cancels_obsolete_parse() {
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

    const PARSE_GAP_DELAY_MS: u64 = 1_500;
    const PARSE_PROGRESS_DELAY_MS: u64 = 1;
    const FIRST_REQUEST_ID: i64 = 50_547;
    const SECOND_REQUEST_ID: i64 = 50_548;
    const SESSION_ID: &str = "file:///current_context_inflight_non_equivalent_fixture.bsl::1";

    let _env_lock = lock_test_env().await;
    let _parse_gap_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_GAP_DELAY_MS.to_string(),
    );
    let _parse_progress_guard = EnvVarGuard::set(
        "BSL_TEST_CURRENT_CONTEXT_PARSE_PROGRESS_DELAY_MS",
        &PARSE_PROGRESS_DELAY_MS.to_string(),
    );

    let mut fixture = String::new();
    for index in 0..1024 {
        fixture.push_str(&format!(
            "Процедура Вспомогательная{}()\n    Сообщить(\"{}\");\nКонецПроцедуры\n\n",
            index, index
        ));
    }
    fixture.push_str(
        "Процедура ЦелеваяПроцедура(Параметр)\n    Сообщить(Параметр);\nКонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri =
        Url::parse("file:///current_context_inflight_non_equivalent_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.clone(),
            },
        })
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let mut current_text = fixture.clone();
    live_transport_append_text_change(
        &mut harness,
        &uri,
        &current_text,
        2,
        "\n// inflight-gap-v2\n",
    )
    .await;
    current_text.push_str("\n// inflight-gap-v2\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe didChange parse gap before in-flight obsolete parse starts");

    let position = find_utf16_position_after_marker(&fixture, "Сообщить(Параметр");
    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 1,
        })],
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if super::super::command_handlers::get_current_context_parse_attempts_for_test() == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("obsolete request must enter parse attempt before newer generation arrives");
    tokio::time::sleep(Duration::from_millis(50)).await;

    live_transport_append_text_change(
        &mut harness,
        &uri,
        &current_text,
        3,
        "\n// inflight-gap-v3\n",
    )
    .await;
    current_text.push_str("\n// inflight-gap-v3\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(3)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe newer revision while obsolete parse is still in flight");

    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 2,
        })],
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(15), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if response_id == FIRST_REQUEST_ID {
                first_response = Some(response);
            } else if response_id == SECOND_REQUEST_ID {
                second_response = Some(response);
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("obsolete and latest in-flight responses must arrive");

    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        None,
        "obsolete in-flight non-equivalent generation must fail closed after cancellation"
    );
    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionKind"))
            .and_then(|value| value.as_str()),
        Some("none"),
        "obsolete in-flight non-equivalent generation must degrade to empty current-context surface"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ЦелеваяПроцедура"),
        "latest non-equivalent generation must still resolve after canceling the obsolete parse"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test(),
        2,
        "obsolete in-flight parse cancellation must allow a fresh newest-generation parse to start"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_cancellations_for_test(),
        1,
        "obsolete in-flight non-equivalent parse must hit the explicit cancellation path"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_superseded")
        ),
        1,
        "obsolete in-flight parse must be visible as superseded in observability"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
        ),
        1,
        "latest in-flight replacement request must still resolve"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_superseded_non_equivalent_generation_skips_obsolete_parse_before_start(
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 900;
    const PARSE_GAP_DELAY_MS: u64 = 1_500;
    const FIRST_REQUEST_ID: i64 = 50_540;
    const SECOND_REQUEST_ID: i64 = 50_541;
    const SESSION_ID: &str = "file:///current_context_latest_only_non_equivalent_fixture.bsl::1";

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );
    let _parse_gap_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &PARSE_GAP_DELAY_MS.to_string(),
    );

    let fixture = concat!(
        "Процедура ПерваяПроцедура(Первый)\n",
        "    Сообщить(Первый);\n",
        "КонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri =
        Url::parse("file:///current_context_latest_only_non_equivalent_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.to_string(),
            },
        })
        .await;
    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let mut current_text = fixture.to_string();
    live_transport_append_text_change(
        &mut harness,
        &uri,
        &current_text,
        2,
        "\n// supersession-gap-v2\n",
    )
    .await;
    current_text.push_str("\n// supersession-gap-v2\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe didChange parse gap before obsolete request starts");

    let position = find_utf16_position_after_marker(fixture, "Сообщить(Первый");
    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 1,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    live_transport_append_text_change(
        &mut harness,
        &uri,
        &current_text,
        3,
        "\n// supersession-gap-v3\n",
    )
    .await;
    current_text.push_str("\n// supersession-gap-v3\n");
    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(3)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test must observe newer revision before latest request starts");

    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": position.line,
            "character": position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 2,
        })],
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if response_id == FIRST_REQUEST_ID {
                first_response = Some(response);
            } else if response_id == SECOND_REQUEST_ID {
                second_response = Some(response);
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("obsolete and latest non-equivalent responses must arrive");

    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        None,
        "obsolete non-equivalent generation must fail closed after a newer revision arrives"
    );
    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionKind"))
            .and_then(|value| value.as_str()),
        Some("none"),
        "obsolete non-equivalent generation must degrade to the empty current-context surface"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ПерваяПроцедура"),
        "latest non-equivalent generation must still resolve against the newest revision"
    );
    assert_eq!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test(),
        1,
        "obsolete non-equivalent generation must be cut off before launching its own expensive parse"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_role_total_role_broker_leader")
        ),
        2,
        "non-equivalent supersession should surface two broker leaders across two revisions"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_superseded")
        ),
        1,
        "obsolete non-equivalent generation must be visible as superseded"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_current_context_terminal_total_outcome_resolved")
        ),
        1,
        "latest non-equivalent generation must still resolve"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_superseded_generation_skips_obsolete_parse_and_stale_surface() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 900;
    const FIRST_REQUEST_ID: i64 = 50_541;
    const SECOND_REQUEST_ID: i64 = 50_542;
    const SESSION_ID: &str = "file:///current_context_latest_only_fixture.bsl::1";

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );

    let fixture = concat!(
        "Процедура ПерваяПроцедура(Первый)\n",
        "    Сообщить(Первый);\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура ВтораяПроцедура(Второй)\n",
        "    Сообщить(Второй);\n",
        "КонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_latest_only_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.to_string(),
            },
        })
        .await;
    server.sync_v2_globals().await;

    let first_position = find_utf16_position_after_marker(fixture, "Сообщить(Первый");
    let second_position = find_utf16_position_after_marker(fixture, "Сообщить(Второй");

    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": first_position.line,
            "character": first_position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 1,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": second_position.line,
            "character": second_position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 2,
        })],
    )
    .await;

    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut first_response = None;
        let mut second_response = None;
        loop {
            let response = harness.read_message().await;
            let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                continue;
            };
            if response_id == FIRST_REQUEST_ID {
                first_response = Some(response);
            } else if response_id == SECOND_REQUEST_ID {
                second_response = Some(response);
            }
            if first_response.is_some() && second_response.is_some() {
                break (
                    first_response.take().expect("first response"),
                    second_response.take().expect("second response"),
                );
            }
        }
    })
    .await
    .expect("both current-context responses must arrive");

    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        None,
        "superseded generation must not surface stale current-context routine name"
    );
    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionKind"))
            .and_then(|value| value.as_str()),
        Some("none"),
        "superseded generation must degrade to empty current-context surface"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ВтораяПроцедура"),
        "latest generation must preserve the newest current-context surface"
    );
    assert!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test() <= 1,
        "superseded generation must not duplicate expensive parse/context derivation after latest-only cutoff"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn p33_get_current_context_superseded_generation_keeps_completion_bounded_under_mixed_load() {
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

    const CURRENT_CONTEXT_DELAY_MS: u64 = 1_500;
    const FIRST_REQUEST_ID: i64 = 50_551;
    const SECOND_REQUEST_ID: i64 = 50_552;
    const COMPLETION_REQUEST_ID: i64 = 50_553;
    const SESSION_ID: &str = "file:///current_context_mixed_load_fixture.bsl::1";
    const RUNTIME_ISOLATION_BUDGET_MS: u64 = 250;

    let _env_lock = lock_test_env().await;
    let _current_context_delay_guard = EnvVarGuard::set(
        "BSL_TEST_GET_CURRENT_CONTEXT_PARSE_DELAY_MS",
        &CURRENT_CONTEXT_DELAY_MS.to_string(),
    );

    let mut fixture = String::new();
    for index in 0..256 {
        fixture.push_str(&format!(
            "Процедура Вспомогательная{}()\n    Сообщить(\"{}\");\nКонецПроцедуры\n\n",
            index, index
        ));
    }
    fixture
        .push_str("Процедура ПерваяПроцедура(Первый)\n    Сообщить(Первый);\nКонецПроцедуры\n\n");
    fixture.push_str(
        "Процедура ВтораяПроцедура(Второй)\n    ДляCompletion = Объект.\n    Сообщить(Второй);\nКонецПроцедуры\n",
    );

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///current_context_mixed_load_fixture.bsl").expect("uri");
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: fixture.clone(),
            },
        })
        .await;
    server.sync_v2_globals().await;

    let first_context_position = find_utf16_position_after_marker(&fixture, "Сообщить(Первый");
    let second_context_position = find_utf16_position_after_marker(&fixture, "Сообщить(Второй");
    let completion_position = find_utf16_position_after_marker(&fixture, "ДляCompletion = Объект.");

    super::super::command_handlers::reset_get_current_context_parse_attempts_for_test();

    live_transport_write_execute_command_request(
        &mut harness,
        FIRST_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": first_context_position.line,
            "character": first_context_position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 1,
        })],
    )
    .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let completion_request_written_at_ms = live_transport_write_completion_request(
        &mut harness,
        COMPLETION_REQUEST_ID,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    live_transport_write_execute_command_request(
        &mut harness,
        SECOND_REQUEST_ID,
        "bsl.getCurrentContext",
        vec![serde_json::json!({
            "uri": uri.to_string(),
            "line": second_context_position.line,
            "character": second_context_position.character,
            "editorSessionId": SESSION_ID,
            "requestGeneration": 2,
        })],
    )
    .await;

    let (completion_response, completion_elapsed_ms, first_response, second_response) =
        tokio::time::timeout(Duration::from_secs(10), async {
            let completion_started = Instant::now();
            let mut completion_response = None;
            let mut completion_elapsed_ms = None;
            let mut first_response = None;
            let mut second_response = None;
            loop {
                let response = harness.read_message().await;
                let Some(response_id) = response.get("id").and_then(|value| value.as_i64()) else {
                    continue;
                };
                if response_id == COMPLETION_REQUEST_ID {
                    completion_elapsed_ms = Some(
                        completion_started
                            .elapsed()
                            .as_millis()
                            .min(u64::MAX as u128) as u64,
                    );
                    completion_response = Some(response);
                } else if response_id == FIRST_REQUEST_ID {
                    first_response = Some(response);
                } else if response_id == SECOND_REQUEST_ID {
                    second_response = Some(response);
                }
                if completion_response.is_some()
                    && first_response.is_some()
                    && second_response.is_some()
                {
                    break (
                        completion_response.take().expect("completion response"),
                        completion_elapsed_ms.expect("completion elapsed"),
                        first_response.take().expect("first response"),
                        second_response.take().expect("second response"),
                    );
                }
            }
        })
        .await
        .expect("completion and current-context responses must arrive");

    assert!(
        completion_response.get("result").is_some(),
        "completion must still return a response while superseded current-context parse is delayed"
    );
    assert!(
        completion_elapsed_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "mixed-load path must keep completion bounded while obsolete current-context work is superseded, completion_elapsed_ms={}ms, completion_response={completion_response:?}",
        completion_elapsed_ms
    );
    assert_eq!(
        first_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        None,
        "mixed-load stale current-context response must stay empty after supersession"
    );
    assert_eq!(
        second_response
            .get("result")
            .and_then(|result| result.get("functionName"))
            .and_then(|value| value.as_str()),
        Some("ВтораяПроцедура"),
        "mixed-load latest current-context response must surface the newest routine"
    );
    assert!(
        super::super::command_handlers::get_current_context_parse_attempts_for_test() <= 1,
        "mixed-load supersession must keep obsolete current-context parse work bounded without duplicate current-context parses"
    );

    let trace = wait_for_live_completion_timeline_trace_with_server_edge_fields(
        &mut harness,
        50_554,
        32,
        COMPLETION_REQUEST_ID,
        &[
            "adapter_read_at_ms",
            "service_future_to_first_poll_wait_ms",
            "response_output_handoff_send_wait_ms",
        ],
    )
    .await;

    let client_to_transport_wait_ms =
        completion_timeline_server_edge_u64(&trace, "adapter_read_at_ms")
            .expect("adapter_read_at_ms")
            .saturating_sub(completion_request_written_at_ms);
    let service_future_to_first_poll_wait_ms =
        completion_timeline_server_edge_u64(&trace, "service_future_to_first_poll_wait_ms")
            .expect("service_future_to_first_poll_wait_ms");
    let response_output_handoff_send_wait_ms =
        completion_timeline_server_edge_u64(&trace, "response_output_handoff_send_wait_ms")
            .expect("response_output_handoff_send_wait_ms");
    assert!(
        client_to_transport_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "mixed-load supersession must not regress completion ingress before adapter_read, client_to_transport_wait_ms={}ms, trace={trace:?}",
        client_to_transport_wait_ms
    );
    assert!(
        service_future_to_first_poll_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "mixed-load supersession must not delay completion first poll, trace={trace:?}"
    );
    assert!(
        response_output_handoff_send_wait_ms <= RUNTIME_ISOLATION_BUDGET_MS,
        "mixed-load supersession must not strand completion response on output handoff, trace={trace:?}"
    );

    live_transport_close_document(&mut harness, &uri).await;
    drop(server);
    harness.shutdown().await;
}

fn message_has_unknown_member(message: &str, member_name: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains(&member_name.to_lowercase())
        && (lower_message.contains("не существует") || lower_message.contains("не найден"))
}

fn message_has_unknown_key(message: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains("ключ") && lower_message.contains("не найден")
}

fn utf16_end_position(source: &str) -> Position {
    let mut line = 0u32;
    let mut last_line = "";
    for (idx, segment) in source.split('\n').enumerate() {
        line = idx as u32;
        last_line = segment;
    }
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    Position::new(line, character)
}

fn histogram_p95(metrics: &serde_json::Value, key: &str) -> f64 {
    metrics
        .get(key)
        .and_then(|value| value.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|n| n as f64)))
        .unwrap_or(0.0)
}

fn histogram_p95_max_by_prefix(metrics: &serde_json::Value, prefix: &str) -> f64 {
    metrics
        .as_object()
        .map(|entries| {
            entries
                .iter()
                .filter(|(key, _)| key.starts_with(prefix) && key.ends_with("_ms"))
                .fold(0.0_f64, |acc, (key, _)| {
                    acc.max(histogram_p95(metrics, key))
                })
        })
        .unwrap_or(0.0)
}

fn dominant_stage_candidate_insert(
    candidates: &mut serde_json::Map<String, serde_json::Value>,
    dominant: &mut Option<(&'static str, f64)>,
    name: &'static str,
    p95: f64,
) {
    candidates.insert(name.to_string(), serde_json::json!(p95));
    if p95 > 0.0 && dominant.is_none_or(|(_, value)| p95 > value) {
        *dominant = Some((name, p95));
    }
}

fn dominant_stage_from_metrics(metrics: &serde_json::Value) -> serde_json::Value {
    let stage_keys = [
        (
            "wait_for_file_version_completion",
            "intellisense_v2_wait_for_file_version_completion_ms",
        ),
        (
            "snapshot_completion",
            "intellisense_v2_snapshot_completion_ms",
        ),
        (
            "ir_query_completion",
            "intellisense_v2_ir_query_completion_ms",
        ),
        (
            "parse_result_query",
            "intellisense_v2_parse_result_query_ms",
        ),
        ("singleflight_wait", "intellisense_v2_singleflight_wait_ms"),
        (
            "runtime_exec_interactive",
            "intellisense_v2_runtime_exec_interactive_ms",
        ),
        (
            "runtime_wait_for_file_version_queue_wait",
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
        ),
        (
            "runtime_snapshot_with_deps_queue_wait",
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
        ),
        (
            "runtime_apply_changes_queue_wait",
            "intellisense_v2_runtime_apply_changes_queue_wait_ms",
        ),
        (
            "runtime_apply_changes_exec",
            "intellisense_v2_runtime_apply_changes_exec_ms",
        ),
        (
            "runtime_apply_change_set_file_exec",
            "intellisense_v2_runtime_apply_change_set_file_exec_ms",
        ),
        (
            "runtime_apply_change_set_file_with_snapshot_exec",
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
        ),
        (
            "runtime_apply_change_remove_file_exec",
            "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
        ),
        (
            "runtime_apply_change_set_settings_snapshot_exec",
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
        ),
        (
            "runtime_type_index_precompute_queue_wait",
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
        ),
        (
            "runtime_type_index_precompute_exec",
            "intellisense_v2_runtime_type_index_precompute_exec_ms",
        ),
        (
            "runtime_type_index_precompute_build_exec",
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
        ),
        (
            "runtime_type_index_precompute_ir_exec",
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
        ),
        (
            "runtime_type_index_precompute_ast_to_ir_exec",
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_seed_module_context_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_local_function_summaries_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_visit_statements_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
        ),
        (
            "completion_stage_turn_wait",
            "completion_stage_turn_wait_ms",
        ),
        (
            "completion_stage_prepare_stateful",
            "completion_stage_prepare_stateful_ms",
        ),
        (
            "completion_stage_prepare_apply_age_at_start",
            "completion_stage_prepare_apply_age_at_start_ms",
        ),
        (
            "completion_stage_prepare_apply_age_at_terminal",
            "completion_stage_prepare_apply_age_at_terminal_ms",
        ),
        (
            "completion_stage_sync_globals",
            "completion_stage_sync_globals_ms",
        ),
        (
            "completion_stage_exact_wait_apply_age_at_start",
            "completion_stage_exact_wait_apply_age_at_start_ms",
        ),
        (
            "completion_stage_exact_wait_apply_age_at_terminal",
            "completion_stage_exact_wait_apply_age_at_terminal_ms",
        ),
        (
            "completion_stage_query_bundle_deps_and_file_snapshot",
            "completion_stage_query_bundle_deps_and_file_snapshot_ms",
        ),
        (
            "completion_stage_query_bundle_pool_wait",
            "completion_stage_query_bundle_pool_wait_ms",
        ),
        (
            "completion_stage_query_bundle_ir_query",
            "completion_stage_query_bundle_ir_query_ms",
        ),
        (
            "completion_stage_query_bundle_ir_retry",
            "completion_stage_query_bundle_ir_retry_ms",
        ),
        (
            "completion_stage_query_bundle_other",
            "completion_stage_query_bundle_other_ms",
        ),
        (
            "completion_stage_response_build",
            "completion_stage_response_build_ms",
        ),
        (
            "completion_stage_cache_store",
            "completion_stage_cache_store_ms",
        ),
        (
            "completion_stage_snapshot_read",
            "completion_stage_snapshot_read_ms",
        ),
        ("completion_stage_collect", "completion_stage_collect_ms"),
        ("completion_stage_rank", "completion_stage_rank_ms"),
        ("completion_stage_format", "completion_stage_format_ms"),
        (
            "runtime_queue_wait_interactive",
            "intellisense_v2_runtime_queue_wait_interactive_ms",
        ),
        (
            "syntax_diagnostics_query",
            "intellisense_v2_syntax_diagnostics_query_ms",
        ),
        (
            "semantic_diagnostics_query",
            "intellisense_v2_semantic_diagnostics_query_ms",
        ),
        (
            "semantic_diagnostics_query_inputs",
            "intellisense_v2_semantic_diagnostics_query_inputs_ms",
        ),
        (
            "semantic_diagnostics_query_parse_result",
            "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
        ),
        (
            "semantic_diagnostics_query_ir",
            "intellisense_v2_semantic_diagnostics_query_ir_ms",
        ),
        (
            "semantic_diagnostics_query_collect",
            "intellisense_v2_semantic_diagnostics_query_collect_ms",
        ),
        (
            "semantic_diagnostics_query_flow_sensitive",
            "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
        ),
    ];

    let mut candidates = serde_json::Map::new();
    let mut dominant: Option<(&'static str, f64)> = None;
    for (name, key) in stage_keys {
        let p95 = histogram_p95(metrics, key);
        dominant_stage_candidate_insert(&mut candidates, &mut dominant, name, p95);
    }
    dominant_stage_candidate_insert(
        &mut candidates,
        &mut dominant,
        "completion_stage_query_bundle_owner_hint",
        histogram_p95_max_by_prefix(metrics, "completion_stage_query_bundle_owner_hint"),
    );

    let (stage, p95_ms) = dominant.unwrap_or(("none", 0.0));
    serde_json::json!({
        "stage": stage,
        "p95_ms": p95_ms,
        "candidates_p95_ms": candidates
    })
}
