#[tokio::test]
async fn p7_signature_help_timeout_still_seeds_exact_type_index_without_did_save() {
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

    const FALLBACK_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";
    const WAIT_BUDGET_EXHAUSTED_KEY: &str =
        "intellisense_v2_interactive_wait_budget_exhausted_total";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK).await;
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );

    let fixture = concat!(
        "Процедура Тест()\n",
        "    МойМассив = Новый Массив();\n",
        "    МойМассив.Добавить(1, 2);\n",
        "КонецПроцедуры\n"
    );
    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p7_signature_help_timeout_still_seeds_exact_without_did_save.bsl",
    )
    .await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let before_metrics = server.coordinator.observability_metrics();
    let before_counters = before_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let before_fail_closed = read_u64_metric(before_counters.get(FALLBACK_REASON_KEY));
    let before_wait_budget_exhausted =
        read_u64_metric(before_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));

    let signature_position = find_utf16_position_after_marker(fixture, "МойМассив.Добавить(1, ");
    let started = Instant::now();
    let first_signature_help = lsp_signature_help_at(&mut service, &uri, signature_position).await;
    let first_elapsed = started.elapsed();
    assert!(
        first_signature_help.is_none(),
        "signatureHelp must remain fail-closed on the first request when same-version exact precompute exceeds the interactive budget"
    );
    assert!(
        first_elapsed
            <= std::time::Duration::from_millis(wait_budget_ms.saturating_add(250).max(250)),
        "signatureHelp timeout must stay bounded by the interactive wait budget, elapsed={first_elapsed:?}, wait_budget_ms={wait_budget_ms}"
    );

    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;
    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_signature_help = lsp_signature_help_at(&mut service, &uri, signature_position)
        .await
        .expect("signatureHelp should succeed after same-version exact precompute finishes");
    let second_signature_label = second_signature_help
        .signatures
        .first()
        .map(|signature| signature.label.as_str())
        .unwrap_or("");
    assert!(
        second_signature_label.contains("Добавить("),
        "signatureHelp must expose exact method signature once same-version exact precompute finishes, label={second_signature_label}"
    );
    assert_eq!(second_signature_help.active_parameter, Some(1));

    let after_metrics = server.coordinator.observability_metrics();
    let after_counters = after_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let after_fail_closed = read_u64_metric(after_counters.get(FALLBACK_REASON_KEY));
    let after_wait_budget_exhausted =
        read_u64_metric(after_counters.get(WAIT_BUDGET_EXHAUSTED_KEY));
    assert!(
        after_fail_closed > before_fail_closed,
        "timed-out signatureHelp must still expose bounded missing_semantic_index attribution"
    );
    assert!(
        after_wait_budget_exhausted > before_wait_budget_exhausted,
        "timed-out signatureHelp bootstrap must attribute the bounded wait budget exhaustion"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_cache_miss_on_map_index_access_does_not_use_legacy_word_fallback() {
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
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
Map = Новый Соответствие;\n\
Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
ЗначДляHover = Map[\"k\"];\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_map_index_no_legacy_fallback.bsl").expect("uri");
    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = ");
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9108)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: hover_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request")
        .expect("hover response");
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    assert!(
        hover.is_none(),
        "hover cache miss on map index access must not synthesize legacy fallback payload: {hover_value:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_legitimate_empty_interactive_results_do_not_emit_fail_closed_reasons() {
    const HOVER_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";
    const SIGNATURE_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";
    const DEFINITION_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
    МойМассив = Новый Массив;\n\
    МойМассив.Несуществующий(1);\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_legitimate_empty_interactive_results.bsl").expect("uri");
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

    let metric_total = |metric_key: &str| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(metric_key))
    };

    let before_hover = metric_total(HOVER_REASON_KEY);
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9111)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(1, 0),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request");
    assert!(
        hover_response.is_some(),
        "hover must return a response envelope even when result is empty"
    );
    let after_hover = metric_total(HOVER_REASON_KEY);
    assert_eq!(
        after_hover, before_hover,
        "legitimate empty hover result must not emit fail-closed reason"
    );

    let before_signature = metric_total(SIGNATURE_REASON_KEY);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9112)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(
                                fixture,
                                "МойМассив.Несуществующий(",
                            ),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request");
    assert!(
        signature_response.is_some(),
        "signatureHelp must return a response envelope even when result is empty"
    );
    let after_signature = metric_total(SIGNATURE_REASON_KEY);
    assert_eq!(
        after_signature, before_signature,
        "unknown method signatureHelp result must not emit fail-closed reason"
    );

    let before_definition = metric_total(DEFINITION_REASON_KEY);
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(9113)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(fixture, "МойМассив."),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request");
    assert!(
        definition_response.is_some(),
        "definition must return a response envelope even when result is empty"
    );
    let after_definition = metric_total(DEFINITION_REASON_KEY);
    assert_eq!(
        after_definition, before_definition,
        "unknown method definition result must not emit fail-closed reason"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_constructor_signature_help_without_canonical_fact_stays_empty_without_fail_closed_reason(
) {
    const SIGNATURE_REASON_KEY: &str = "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n\
    Новый Массив(1, )\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_constructor_signature_help_without_canonical_fact.bsl")
        .expect("uri");
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

    let metric_total = |metric_key: &str| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(metric_key))
    };

    let before_signature = metric_total(SIGNATURE_REASON_KEY);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9114)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(fixture, "Новый Массив("),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request")
        .expect("signatureHelp response");
    let signature_value =
        serde_json::to_value(&signature_response).expect("serialize signatureHelp response");
    let signature_result = signature_value
        .get("result")
        .cloned()
        .expect("signatureHelp result field");
    let signature_help: Option<tower_lsp::lsp_types::SignatureHelp> =
        serde_json::from_value(signature_result).expect("parse signatureHelp result");
    assert!(
        signature_help.is_none(),
        "constructor signatureHelp without canonical fact must stay empty on the default LSP path: {signature_value:?}"
    );
    let after_signature = metric_total(SIGNATURE_REASON_KEY);
    assert_eq!(
        after_signature, before_signature,
        "constructor signatureHelp without canonical fact must remain a legitimate empty result"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_map_index_access_exact_cross_consumer_acceptance_uses_snapshot_owner_without_manual_hint(
) {
    let completion_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    ДляCompletion = Map[\"k\"].\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    ДляHover = Map[\"k\"];\n\
    Проверка = Map[\"k\"].Колонки;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_universal_map_exact_acceptance.bsl",
    )
    .await;

    let legacy_type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        legacy_type_name, "ТаблицаЗначений",
        "legacy type-at-position must already know map value type before completion"
    );

    let serve_only_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        serve_only_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "serve-only snapshot contract must already know map value type before completion"
    );

    let completion_labels = lsp_completion_labels_at(
        &mut service,
        &uri,
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Map[\"k\"]."),
    )
    .await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "completion must expose map value members, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "Ключ" || label == "Значение"),
        "completion must not fall back to key/value pair members, labels={completion_labels:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let serve_only_hover_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        serve_only_hover_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "resolved map index must already have exact serve-only type before hover"
    );

    let hover_text = lsp_hover_text_at(
        &mut service,
        &uri,
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Map[\"k\"]"),
    )
    .await;
    assert!(
        hover_text.contains("ТаблицаЗначений"),
        "hover must expose resolved map value type, hover={hover_text}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, resolved_fixture, "ДляHover = Map[\"k\"]")
            .await;
    assert_eq!(
        type_name, "ТаблицаЗначений",
        "type-at-position must match the resolved map value type"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Колонки")),
        "diagnostics must not drift for known map value member, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_dynamic_map_key_exact_cross_consumer_acceptance_uses_safe_policy_without_unknown_key() {
    let completion_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    Ключ = \"k\";\n\
    ДляCompletion = Map[Ключ].\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    Ключ = \"k\";\n\
    ДляHover = Map[Ключ];\n\
    Проверка = Map[Ключ].Колонки;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_universal_map_dynamic_exact_acceptance.bsl",
    )
    .await;

    let completion_labels = lsp_completion_labels_at(
        &mut service,
        &uri,
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Map[Ключ]."),
    )
    .await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "completion must expose generic map value members for dynamic key, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "Ключ" || label == "Значение"),
        "dynamic map key must not complete as key/value pair, labels={completion_labels:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let serve_only_hover_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Map[Ключ]",
    )
    .await;
    assert_eq!(
        serve_only_hover_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "resolved dynamic map index must already have generic serve-only value type before hover"
    );

    let hover_text = lsp_hover_text_at(
        &mut service,
        &uri,
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Map[Ключ]"),
    )
    .await;
    assert!(
        hover_text.contains("ТаблицаЗначений"),
        "hover must expose generic map value type for dynamic key, hover={hover_text}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, resolved_fixture, "ДляHover = Map[Ключ]")
            .await;
    assert_eq!(
        type_name, "ТаблицаЗначений",
        "dynamic key type-at-position must follow generic value contract"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Колонки")),
        "diagnostics must not drift for known dynamic map value member, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_key(message)),
        "dynamic map key must not emit unknown-key diagnostics, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics(
) {
    let completion_fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    S.Вставить(\"Количество\", 10);\n\
    ДляCompletion = S.\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    S.Вставить(\"Количество\", 10);\n\
    ДляHover = S.Идентификатор;\n\
    Ошибка = S.Идентифкатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_typed_structure_exact_acceptance.bsl",
    )
    .await;

    let completion_position =
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = S.");
    let completion_members =
        lsp_completion_members_at(&mut service, &uri, completion_position).await;
    let completion_labels = completion_members
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "completion must include typed structure field Идентификатор, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "completion must include typed structure field Количество, labels={completion_labels:?}"
    );

    let runtime_resolution = snapshot_type_resolution_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = S",
    )
    .await;
    let runtime_identifier_identity = runtime_resolution
        .find_structural_member("Идентификатор")
        .map(|member| member.member_id.key.clone())
        .expect("runtime structural identity for Идентификатор");
    let runtime_quantity_identity = runtime_resolution
        .find_structural_member("Количество")
        .map(|member| member.member_id.key.clone())
        .expect("runtime structural identity for Количество");

    let lsp_identifier = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("lsp completion entry Идентификатор");
    let lsp_quantity = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("lsp completion entry Количество");
    assert_eq!(
        lsp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "LSP completion must expose the same structural member identity as runtime"
    );
    assert_eq!(
        lsp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "LSP completion must expose the same quantity member identity as runtime"
    );

    let mcp_members = mcp_member_entries_at_code(completion_fixture, completion_position).await;
    let mcp_identifier = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("mcp members entry Идентификатор");
    let mcp_quantity = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("mcp members entry Количество");
    assert_eq!(
        mcp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "MCP members must expose the same structural member identity as runtime"
    );
    assert_eq!(
        mcp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "MCP members must expose the same quantity member identity as runtime"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let resolved_position =
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_at(&mut service, &uri, resolved_position).await;
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose structure field name and type, hover={hover_text}"
    );

    let type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = S.Идентификатор",
    )
    .await;
    assert_eq!(
        type_name, "Строка",
        "typed structure type-at-position must expose field type"
    );

    let mcp_type_name = mcp_type_name_at_code(resolved_fixture, resolved_position).await;
    assert_eq!(
        mcp_type_name, type_name,
        "MCP type_at_position must agree with shared runtime type for typed structure field"
    );

    let web_hover_text = web_hover_text_for_code(resolved_fixture, resolved_position).await;
    assert!(
        web_hover_text.contains(&type_name),
        "Web hover must agree with shared runtime type for typed structure field, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентифкатор")),
        "typed structure typo must produce unknown-member diagnostic, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Идентификатор")),
        "typed structure exact field must not regress to unknown-member diagnostic, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics(
) {
    let completion_fixture = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    ТЗ.Колонки.Добавить(\"Количество\", Новый ОписаниеТипов(\"Число\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    ТЗ.Колонки.Добавить(\"Количество\", Новый ОписаниеТипов(\"Число\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляHover = Стр.Идентификатор;\n\
    Ошибка = Стр.Идентифкатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_typed_value_table_row_exact_acceptance.bsl",
    )
    .await;

    let completion_position =
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Стр.");
    let completion_members =
        lsp_completion_members_at(&mut service, &uri, completion_position).await;
    let completion_labels = completion_members
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "completion must include typed-row column Идентификатор, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "completion must include typed-row column Количество, labels={completion_labels:?}"
    );

    let runtime_resolution = snapshot_type_resolution_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Стр",
    )
    .await;
    let runtime_identifier_identity = runtime_resolution
        .find_structural_member("Идентификатор")
        .map(|member| member.member_id.key.clone())
        .expect("runtime typed-row identity for Идентификатор");
    let runtime_quantity_identity = runtime_resolution
        .find_structural_member("Количество")
        .map(|member| member.member_id.key.clone())
        .expect("runtime typed-row identity for Количество");

    let lsp_identifier = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("lsp completion entry Идентификатор");
    let lsp_quantity = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("lsp completion entry Количество");
    assert_eq!(
        lsp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "LSP completion must expose the same typed-row member identity as runtime"
    );
    assert_eq!(
        lsp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "LSP completion must expose the same typed-row quantity identity as runtime"
    );

    let mcp_members = mcp_member_entries_at_code(completion_fixture, completion_position).await;
    let mcp_identifier = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("mcp members entry Идентификатор");
    let mcp_quantity = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("mcp members entry Количество");
    assert_eq!(
        mcp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "MCP members must expose the same typed-row member identity as runtime"
    );
    assert_eq!(
        mcp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "MCP members must expose the same typed-row quantity identity as runtime"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let resolved_position =
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Стр.Идентификатор");
    let hover_text = lsp_hover_text_at(&mut service, &uri, resolved_position).await;
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose typed-row column name and type, hover={hover_text}"
    );

    let type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Стр.Идентификатор",
    )
    .await;
    assert_eq!(
        type_name, "Строка",
        "typed-row type-at-position must expose column type"
    );

    let mcp_type_name = mcp_type_name_at_code(resolved_fixture, resolved_position).await;
    assert_eq!(
        mcp_type_name, type_name,
        "MCP type_at_position must agree with shared runtime type for typed-row field"
    );

    let web_hover_text = web_hover_text_for_code(resolved_fixture, resolved_position).await;
    assert!(
        web_hover_text.contains(&type_name),
        "Web hover must agree with shared runtime type for typed-row field, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентифкатор")),
        "typed-row typo must produce unknown-member diagnostic, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Идентификатор")),
        "typed-row exact column must not regress to unknown-member diagnostic, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_form_module_object_completion_uses_default_lsp_owner_hint_path() {
    let fixture = "Процедура Тест()\n\
    ДляCompletion = Объект.\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
    )
    .await;

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "default LSP completion for FormModule.Объект must include form-data property Ссылка, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "ПометкаУдаления"),
        "default LSP completion for FormModule.Объект must include form-data property ПометкаУдаления, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "ПолучитьСсылкуНового"),
        "default LSP completion for FormModule.Объект must not expose applied object-facet method ПолучитьСсылкуНового, labels={completion_labels:?}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, fixture, "ДляCompletion = Объект").await;
    assert_eq!(
        type_name, "ДанныеФормыСтруктура",
        "default LSP path must keep shared form-data type at implicit Объект"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces(
) {
    let fixture_v1 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляCompletion = S.\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    ДляCompletion = S.\n\
    Ошибка = S.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_typed_structure_revision_switch.bsl",
    )
    .await;

    let v1_completion_position = find_utf16_position_after_marker(fixture_v1, "ДляCompletion = S.");
    let v1_completion_members =
        lsp_completion_members_at(&mut service, &uri, v1_completion_position).await;
    assert!(
        v1_completion_members.iter().any(|entry| {
            entry.name == "Идентификатор" && entry.member_identity.is_some()
        }),
        "v1 completion must expose structural member identity before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let runtime_resolution =
        snapshot_type_resolution_at_marker(&server, file_id, fixture_v2, "ДляCompletion = S").await;
    assert!(
        runtime_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "runtime snapshot after revision switch must not leak stale structure field"
    );

    let v2_completion_position = find_utf16_position_after_marker(fixture_v2, "ДляCompletion = S.");
    let v2_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, v2_completion_position).await;
    assert!(
        !v2_completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "LSP completion must fail closed after revision switch, labels={v2_completion_labels:?}"
    );

    let mcp_members = mcp_member_entries_at_code(fixture_v2, v2_completion_position).await;
    assert!(
        !mcp_members
            .iter()
            .any(|entry| entry.name == "Идентификатор"),
        "MCP members must not leak stale structure field after revision switch, members={mcp_members:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "runtime/LSP diagnostics must surface stale structure field as unknown-member, diagnostics={diagnostics:?}"
    );

    let web_diagnostics = web_semantic_diagnostic_messages_for_code(fixture_v2).await;
    assert!(
        web_diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "Web diagnostics must surface stale structure field as unknown-member, diagnostics={web_diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces(
) {
    let fixture_v1 = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
    Ошибка = Стр.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_typed_value_table_row_revision_switch.bsl",
    )
    .await;

    let v1_completion_position =
        find_utf16_position_after_marker(fixture_v1, "ДляCompletion = Стр.");
    let v1_completion_members =
        lsp_completion_members_at(&mut service, &uri, v1_completion_position).await;
    assert!(
        v1_completion_members.iter().any(|entry| {
            entry.name == "Идентификатор" && entry.member_identity.is_some()
        }),
        "v1 typed-row completion must expose structural member identity before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let runtime_resolution =
        snapshot_type_resolution_at_marker(&server, file_id, fixture_v2, "ДляCompletion = Стр")
            .await;
    assert!(
        runtime_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "runtime snapshot after revision switch must not leak stale typed-row column"
    );

    let v2_completion_position =
        find_utf16_position_after_marker(fixture_v2, "ДляCompletion = Стр.");
    let v2_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, v2_completion_position).await;
    assert!(
        !v2_completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "LSP completion must fail closed after typed-row revision switch, labels={v2_completion_labels:?}"
    );

    let mcp_members = mcp_member_entries_at_code(fixture_v2, v2_completion_position).await;
    assert!(
        !mcp_members
            .iter()
            .any(|entry| entry.name == "Идентификатор"),
        "MCP members must not leak stale typed-row column after revision switch, members={mcp_members:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "runtime/LSP diagnostics must surface stale typed-row column as unknown-member, diagnostics={diagnostics:?}"
    );

    let web_diagnostics = web_semantic_diagnostic_messages_for_code(fixture_v2).await;
    assert!(
        web_diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "Web diagnostics must surface stale typed-row column as unknown-member, diagnostics={web_diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member()
{
    let fixture_v1 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_hover_type_revision_switch_structure.bsl",
    )
    .await;

    let v1_position = find_utf16_position_at_marker_tail(fixture_v1, "ДляHover = S.Идентификатор");
    let v1_hover_text = lsp_hover_text_optional_at(&mut service, &uri, v1_position)
        .await
        .expect("v1 hover text");
    assert!(
        v1_hover_text.contains("Идентификатор") && v1_hover_text.contains("Строка"),
        "v1 hover must expose the exact typed structure field before revision switch, hover={v1_hover_text}"
    );
    let v1_type_name = snapshot_type_name_at_marker_optional(
        &server,
        file_id,
        fixture_v1,
        "ДляHover = S.Идентификатор",
    )
    .await
    .expect("v1 type_at_position");
    assert_eq!(
        v1_type_name, "Строка",
        "v1 type_at_position must expose the exact typed structure field before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let v2_position = find_utf16_position_at_marker_tail(fixture_v2, "ДляHover = S.Идентификатор");
    let v2_hover_text = lsp_hover_text_optional_at(&mut service, &uri, v2_position).await;
    if let Some(text) = &v2_hover_text {
        assert!(
            !text.contains("Строка"),
            "LSP hover must not leak stale previous-revision field type after revision switch, hover={text}"
        );
        assert!(
            text.contains("Неопределено") || text.contains("Тип не распознан системой"),
            "non-empty LSP hover after revision switch must describe the current unresolved state instead of stale field semantics, hover={text}"
        );
    }

    let v2_type_name = snapshot_type_name_at_marker_optional(
        &server,
        file_id,
        fixture_v2,
        "ДляHover = S.Идентификатор",
    )
    .await;
    assert_ne!(
        v2_type_name.as_deref(),
        Some("Строка"),
        "runtime type_at_position must not leak stale previous-revision field type after revision switch, type={v2_type_name:?}"
    );

    let web_hover_text = web_hover_text_for_code(fixture_v2, v2_position).await;
    assert!(
        !web_hover_text.contains("Строка"),
        "Web hover must not leak stale previous-revision field type after revision switch, hover={web_hover_text}"
    );
    assert!(
        web_hover_text.is_empty()
            || web_hover_text.contains("Неопределено")
            || web_hover_text.contains("Тип не распознан системой"),
        "non-empty Web hover after revision switch must describe the current unresolved state instead of stale field semantics, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "revision-switched typed structure access must produce unknown-member diagnostics, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp(
) {
    let fixture_v1 = "Процедура Целевой()\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_definition_revision_switch_removed_local_target.bsl",
    )
    .await;

    let v1_position = find_utf16_position_after_marker(fixture_v1, "Процедура Тест()\nЦелевой");
    let v1_lsp_definition = lsp_definition_points_at(&mut service, &uri, v1_position).await;
    assert!(
        !v1_lsp_definition.is_empty(),
        "v1 definition must resolve before revision switch, definition={v1_lsp_definition:?}"
    );
    let v1_mcp_definition = mcp_definition_points_at_code(fixture_v1, v1_position).await;
    assert!(
        !v1_mcp_definition.is_empty(),
        "v1 MCP definition must resolve before revision switch, definition={v1_mcp_definition:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let v2_position = find_utf16_position_after_marker(fixture_v2, "Процедура Тест()\nЦелевой");
    let v2_lsp_definition = lsp_definition_points_at(&mut service, &uri, v2_position).await;
    assert!(
        v2_lsp_definition.is_empty(),
        "LSP definition must not leak stale previous-revision target location after revision switch, definition={v2_lsp_definition:?}"
    );

    let v2_mcp_definition = mcp_definition_points_at_code(fixture_v2, v2_position).await;
    assert!(
        v2_mcp_definition.is_empty(),
        "MCP definition must not leak stale previous-revision target location for the current code, definition={v2_mcp_definition:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        !diagnostics.is_empty(),
        "removed local target must surface current-revision diagnostics instead of silently reusing stale semantics"
    );

    drain_task.abort();
}
