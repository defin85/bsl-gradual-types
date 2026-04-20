#[tokio::test]
async fn p26_interactive_warm_path_completion_slo_smoke_conf_big() {
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let Some(root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p26 warm-path SLO smoke: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p26 warm-path SLO smoke: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly",
            module_path.display()
        );
    }

    let module_text = std::fs::read_to_string(&module_path).expect("read conf_big module");
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

    let uri = Url::parse("file:///conf_big_perf_module.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: module_text,
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

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    for _ in 0..50_u64 {
        let completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion request");
        assert!(completion.is_some(), "completion response expected");
    }

    // Dedicated concurrent parse burst to exercise parse_result singleflight sharing
    // without polluting completion duration SLO samples.
    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file_id must be available after didOpen");
    let parse_context = Arc::new(
        server
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                file_id,
                None,
                false,
            )
            .await,
    );
    let parse_barrier = Arc::new(std::sync::Barrier::new(9));
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..8_u32 {
            let runtime = server.analysis_v2.clone();
            let parse_context = parse_context.clone();
            let parse_barrier = parse_barrier.clone();
            let coordinator = coordinator.clone();
            workers.push(scope.spawn(move || {
                parse_barrier.wait();
                let analysis = futures::executor::block_on(runtime.snapshot());
                bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                    parse_context.as_ref(),
                    &analysis,
                    true,
                    Some(coordinator.as_ref()),
                    file_id,
                )
            }));
        }
        parse_barrier.wait();
        for worker in workers {
            let result = worker.join().expect("parse burst worker should not panic");
            assert!(
                result.is_ok(),
                "parse burst worker should complete without hard cancellation"
            );
        }
    });

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let rates = metrics
        .get("rates")
        .and_then(|value| value.as_object())
        .expect("metrics.rates object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let wait_hist = histograms
        .get("intellisense_v2_wait_for_file_version_completion_ms")
        .or_else(|| histograms.get("intellisense_v2_wait_for_file_version_other_ms"))
        .and_then(|value| value.as_object());
    let completion_hist = histograms
        .get("completion_duration_ms")
        .and_then(|value| value.as_object())
        .expect("completion duration histogram");

    let completion_count = completion_hist
        .get("count")
        .and_then(|value| value.as_u64())
        .expect("completion count");
    assert!(
        completion_count >= 50,
        "expected at least 50 completion duration samples, got {completion_count}"
    );

    let wait_p95 = wait_hist
        .and_then(|hist| hist.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let queue_wait_interactive_p95 = histograms
        .get("intellisense_v2_runtime_queue_wait_interactive_ms")
        .and_then(|value| value.as_object())
        .and_then(|hist| hist.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let completion_p95 = completion_hist
        .get("p95")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .expect("completion p95");
    let parse_result_shared_rate = rates
        .get("intellisense_v2_parse_result_singleflight_shared_rate")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let parse_result_cancel_rate = rates
        .get("intellisense_v2_parse_result_query_cancel_rate")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120)
        .clamp(10, 2000) as f64;

    assert!(
        wait_p95 <= wait_budget_ms + 20.0,
        "warm-path wait p95 regression: wait_p95={}ms budget={}ms",
        wait_p95,
        wait_budget_ms
    );
    assert!(
        completion_p95 <= 1500.0,
        "warm-path completion p95 regression: completion_p95={}ms > 1500ms",
        completion_p95
    );
    assert!(
        queue_wait_interactive_p95 <= wait_budget_ms + 250.0,
        "warm-path interactive queue-wait p95 regression: queue_wait_interactive_p95={}ms budget={}ms",
        queue_wait_interactive_p95,
        wait_budget_ms
    );
    assert!(
        parse_result_shared_rate >= 0.01,
        "parse_result singleflight shared-rate regression: shared_rate={:.3}",
        parse_result_shared_rate
    );
    assert!(
        parse_result_cancel_rate <= 0.30,
        "parse_result cancel-rate regression: cancel_rate={:.3}",
        parse_result_cancel_rate
    );
    let completion_total = counters
        .get("completion_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        completion_total >= 50,
        "expected completion_total >= 50, got {completion_total}"
    );
    let completion_cancelled_total = counters
        .get("intellisense_v2_completion_result_total_cancelled")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let completion_cancelled_rate =
        completion_cancelled_total as f64 / completion_total.max(1) as f64;
    assert!(
        completion_cancelled_rate <= 0.10,
        "warm-path completion cancel-rate regression: cancelled={} total={} rate={:.3}",
        completion_cancelled_total,
        completion_total,
        completion_cancelled_rate
    );

    drain_task.abort();
}

#[tokio::test]
async fn p32_foreign_document_skips_config_semantic_diagnostics() {
    fn write_configuration_xml(root: &std::path::Path, name: &str, common_modules: &[&str]) {
        let mut child_objects = String::new();
        for module in common_modules {
            child_objects.push_str(&format!("<CommonModule>{module}</CommonModule>"));
        }
        std::fs::write(
            root.join("Configuration.xml"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>{name}</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>{child_objects}</ChildObjects>
  </Configuration>
</MetaDataObject>
"#
            ),
        )
        .expect("write Configuration.xml");
    }

    fn write_common_module(root: &std::path::Path, name: &str) {
        std::fs::create_dir_all(root.join("CommonModules")).expect("create CommonModules");
        std::fs::write(
            root.join("CommonModules").join(format!("{name}.xml")),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>{name}</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#
            ),
        )
        .expect("write CommonModule xml");
        std::fs::create_dir_all(root.join("CommonModules").join(name).join("Ext"))
            .expect("create common module dir");
        std::fs::write(
            root.join("CommonModules")
                .join(name)
                .join("Ext")
                .join("Module.bsl"),
            "Процедура Ф() Экспорт\nКонецПроцедуры\n",
        )
        .expect("write common module source");
    }

    fn write_form_module(
        root: &std::path::Path,
        document_name: &str,
        form_name: &str,
        code: &str,
    ) -> std::path::PathBuf {
        let path = root
            .join("Documents")
            .join(document_name)
            .join("Forms")
            .join(form_name)
            .join("Ext")
            .join("Form")
            .join("Module.bsl");
        std::fs::create_dir_all(path.parent().expect("form module parent"))
            .expect("create form module dir");
        std::fs::write(&path, code).expect("write form module source");
        path
    }

    let tmp = tempfile::TempDir::new().expect("temp dir");
    let configured_root = tmp.path().join("configured");
    let foreign_root = tmp.path().join("foreign");
    std::fs::create_dir_all(&configured_root).expect("create configured root");
    std::fs::create_dir_all(&foreign_root).expect("create foreign root");

    write_configuration_xml(&configured_root, "ConfiguredConfig", &[]);
    write_configuration_xml(&foreign_root, "ForeignConfig", &["МойМодуль"]);
    write_common_module(&foreign_root, "МойМодуль");

    let configured_form_code = "Процедура Тест()\n    НесуществующийМодуль.Ф();\nКонецПроцедуры\n";
    let configured_form_path =
        write_form_module(&configured_root, "Док1", "Форма1", configured_form_code);
    let foreign_form_code = "Процедура Тест()\n    МойМодуль.Ф();\nКонецПроцедуры\n";
    let foreign_form_path = write_form_module(&foreign_root, "Док2", "Форма2", foreign_form_code);

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
        .expect("server must be created");
    *server.config.write().await = Some(crate::config::LspConfig {
        platform_docs_archive: None,
        configuration_path: Some(configured_root.to_string_lossy().to_string()),
        platform_version: Some("8.3.25".to_string()),
        cache_enabled: Some(true),
        strict_fingerprint: Some(false),
        enable_type_hints: Some(false),
        enable_code_actions: Some(false),
    });

    let startup_coordinator = coordinator.clone();
    let startup_root = configured_root.clone();
    tokio::task::spawn_blocking(move || {
        startup_coordinator.start_with_paths_blocking(
            None,
            Some(&startup_root),
            Some("8.3.25"),
            None,
        )
    })
    .await
    .expect("config startup join")
    .expect("config startup");
    server
        .deps_update_v2(
            "p32_foreign_document_skips_config_semantic_diagnostics",
            None,
            Some(configured_root.clone()),
        )
        .await;

    let configured_uri =
        Url::from_file_path(&configured_form_path).expect("configured form module uri");
    let configured_did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: configured_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: configured_form_code.to_string(),
        },
    };
    let configured_open_req = Request::build("textDocument/didOpen")
        .params(
            serde_json::to_value(configured_did_open)
                .expect("configured DidOpenTextDocumentParams"),
        )
        .finish();
    let configured_open_response = service
        .ready()
        .await
        .unwrap()
        .call(configured_open_req)
        .await
        .expect("configured didOpen notification");
    assert!(
        configured_open_response.is_none(),
        "didOpen is a notification"
    );

    let configured_diagnostics =
        wait_lsp_publish_diagnostics(&mut published_rx, &configured_uri).await;
    assert!(
        configured_diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("Необъявленная переменная")
                && diagnostic.message.contains("НесуществующийМодуль")
        }),
        "expected semantic diagnostic for file inside configured root, got {:?}",
        configured_diagnostics
    );

    let foreign_uri = Url::from_file_path(&foreign_form_path).expect("foreign form module uri");
    let foreign_did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: foreign_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: foreign_form_code.to_string(),
        },
    };
    let foreign_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(foreign_did_open).expect("foreign DidOpenTextDocumentParams"))
        .finish();
    let foreign_open_response = service
        .ready()
        .await
        .unwrap()
        .call(foreign_open_req)
        .await
        .expect("foreign didOpen notification");
    assert!(foreign_open_response.is_none(), "didOpen is a notification");

    let foreign_diagnostics = wait_any_lsp_publish_diagnostics(
        &mut published_rx,
        &foreign_uri,
        tokio::time::Duration::from_secs(5),
    )
    .await
    .expect("foreign diagnostics publish");
    assert!(
        !foreign_diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("Необъявленная переменная")
                && diagnostic.message.contains("МойМодуль")
        }),
        "expected semantic stage to be skipped for file outside configured root, got {:?}",
        foreign_diagnostics
    );

    drain_task.abort();
}

#[tokio::test]
async fn p27_interactive_completion_acceptance_gates_emit_artifact() {
    const CHANGE_ID: &str = "refactor-ir-canonical-semantic-pipeline";
    const ITERATIONS: u64 = 120;
    const MAX_P95_MS: f64 = 300.0;
    const MAX_P99_MS: f64 = 800.0;
    const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;
    const MAX_TERMINAL_EMPTY_RATE: f64 = 0.005;
    const MAX_PARITY_MISMATCH_RATE: f64 = 0.01;

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn sum_counters_by_prefix(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn sum_counters_by_prefix_and_substring(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
        needle: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix) && key.contains(needle))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn stage_mode_counter_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        stage: &str,
        mode: &str,
    ) -> u64 {
        let stage_token = format!("_stage_{stage}");
        counters
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_total_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                    && key.contains(&format!("_mode_{mode}"))
            })
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn stage_mode_latency_p95(
        histograms: &serde_json::Map<String, serde_json::Value>,
        stage: &str,
        mode: &str,
    ) -> f64 {
        let stage_token = format!("_stage_{stage}");
        histograms
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_latency_ms_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                    && key.contains(&format!("_mode_{mode}"))
            })
            .filter_map(|(_, value)| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .fold(0.0, f64::max)
    }

    fn collect_mode_split_stage_metrics(
        counters: &serde_json::Map<String, serde_json::Value>,
        histograms: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        const STAGES: &[&str] = &[
            "runtime_wait_for_file_version",
            "runtime_snapshot_with_deps",
            "ir_query",
            "parse_result_query",
        ];
        const MODES: &[&str] = &["legacy", "event_driven", "shadow"];

        let mut by_mode = serde_json::Map::new();
        for mode in MODES {
            let mut by_stage = serde_json::Map::new();
            for stage in STAGES {
                by_stage.insert(
                    (*stage).to_string(),
                    serde_json::json!({
                        "total": stage_mode_counter_total(counters, stage, mode),
                        "p95_ms": stage_mode_latency_p95(histograms, stage, mode),
                    }),
                );
            }
            by_mode.insert((*mode).to_string(), serde_json::Value::Object(by_stage));
        }
        serde_json::Value::Object(by_mode)
    }

    fn parse_snapshot_mode_counter_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> u64 {
        counters
            .get(&format!(
                "intellisense_v2_parse_snapshot_total_origin_lsp_mode_{mode}"
            ))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    }

    fn parse_snapshot_mode_latency_p95(
        histograms: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> f64 {
        histograms
            .get(&format!(
                "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_{mode}"
            ))
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0)
    }

    fn collect_parse_snapshot_mode_metrics(
        counters: &serde_json::Map<String, serde_json::Value>,
        histograms: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        const PARSE_MODES: &[&str] = &["incremental", "reused", "full", "other"];

        let changed_ranges_count_p95 = histograms
            .get("intellisense_v2_parse_snapshot_changed_ranges_count_origin_lsp")
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0);
        let changed_ranges_bytes_p95 = histograms
            .get("intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_lsp")
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0);

        let mut by_mode = serde_json::Map::new();
        for mode in PARSE_MODES {
            by_mode.insert(
                (*mode).to_string(),
                serde_json::json!({
                    "total": parse_snapshot_mode_counter_total(counters, mode),
                    "p95_ms": parse_snapshot_mode_latency_p95(histograms, mode),
                }),
            );
        }

        serde_json::json!({
            "by_mode": by_mode,
            "changed_ranges_count_p95": changed_ranges_count_p95,
            "changed_ranges_bytes_p95": changed_ranges_bytes_p95,
        })
    }

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
        .expect("server must be created");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p27_interactive_acceptance_gate.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    ЛокМассив = Новый Массив;\n",
        "    ЛокМассив.\n",
        "КонецПроцедуры\n"
    );
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
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
    let member_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

    let mut first_trigger_success_total = 0_u64;
    let mut first_trigger_total = 0_u64;
    let mut parity_pairs_total = 0_u64;

    for iteration in 0..ITERATIONS {
        let version = (iteration + 2) as i32;
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
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

        let dot_completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: Some(".".to_string()),
                }),
            })
            .await
            .expect("dot completion request")
            .expect("dot completion response");
        first_trigger_total += 1;
        if completion_items_count(&dot_completion) > 0 {
            first_trigger_success_total += 1;
        }

        let invoked_completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })
            .await
            .expect("invoked completion request")
            .expect("invoked completion response");
        assert!(
            completion_items_count(&invoked_completion) > 0,
            "invoked completion must return non-empty candidates in acceptance gate loop"
        );
        parity_pairs_total += 1;
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let completion_hist = histograms
        .get("completion_duration_ms")
        .and_then(|value| value.as_object())
        .expect("completion duration histogram");
    let completion_p95 = metric_as_f64(completion_hist.get("p95"));
    let completion_p99 = metric_as_f64(completion_hist.get("p99"));
    let mode_split_stage_metrics = collect_mode_split_stage_metrics(counters, histograms);
    let parse_snapshot_mode_metrics = collect_parse_snapshot_mode_metrics(counters, histograms);

    let first_trigger_success_rate =
        first_trigger_success_total as f64 / first_trigger_total.max(1) as f64;
    let terminal_empty_fail_closed_total = [
        "missing_canonical_ir",
        "missing_semantic_index",
        "superseded_revision",
        "cancelled",
        "unavailable_by_contract",
    ]
    .into_iter()
    .map(|reason| {
        sum_counters_by_prefix_and_substring(
            counters,
            "intellisense_v2_completion_member_access_terminal_empty_total_",
            &format!("_reason_{reason}"),
        )
    })
    .sum::<u64>();
    let terminal_empty_rate =
        terminal_empty_fail_closed_total as f64 / first_trigger_total.max(1) as f64;
    let parity_drift_total = sum_counters_by_prefix(
        counters,
        "intellisense_v2_completion_parity_drift_total_mode_",
    );
    let parity_mismatch_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
    let parity_evidence = serde_json::json!({
        "results": {
            "parity_pairs_total": parity_pairs_total,
            "parity_mismatch_rate": parity_mismatch_rate
        }
    });
    let parity_evidence_verdict = validate_parity_cutover_evidence(&parity_evidence);
    let completion_mode = bsl_runtime::system::global_runtime_config()
        .get_string(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionMode)
        .unwrap_or_else(|| "on".to_string())
        .to_ascii_lowercase();
    let canary_percent = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionCanaryPercent)
        .unwrap_or(0)
        .clamp(0, 100) as u8;
    let report_mode_suffix = if completion_mode == "canary" {
        format!("canary-{canary_percent}")
    } else {
        completion_mode.clone()
    };

    let pass = completion_p95 <= MAX_P95_MS
        && completion_p99 <= MAX_P99_MS
        && first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE
        && terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE
        && parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE
        && parity_evidence_verdict.is_ok();

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p27_interactive_completion_acceptance_gates",
        "mode": completion_mode,
        "canary_percent": canary_percent,
        "iterations": ITERATIONS,
        "thresholds": {
            "completion_p95_ms_max": MAX_P95_MS,
            "completion_p99_ms_max": MAX_P99_MS,
            "first_trigger_success_rate_min": MIN_FIRST_TRIGGER_SUCCESS_RATE,
            "terminal_empty_fail_closed_rate_max": MAX_TERMINAL_EMPTY_RATE,
            "parity_mismatch_rate_max": MAX_PARITY_MISMATCH_RATE,
            "parity_pairs_total_min": PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
            "parity_drift_rate_max": PARITY_DRIFT_RATE_MAX_FOR_CUTOVER
        },
        "results": {
            "completion_p95_ms": completion_p95,
            "completion_p99_ms": completion_p99,
            "first_trigger_success_total": first_trigger_success_total,
            "first_trigger_total": first_trigger_total,
            "first_trigger_success_rate": first_trigger_success_rate,
            "terminal_empty_fail_closed_total": terminal_empty_fail_closed_total,
            "terminal_empty_fail_closed_rate": terminal_empty_rate,
            "parity_drift_total": parity_drift_total,
            "parity_pairs_total": parity_pairs_total,
            "parity_mismatch_rate": parity_mismatch_rate,
            "mode_split_stage_metrics": mode_split_stage_metrics,
            "parse_snapshot_mode_metrics": parse_snapshot_mode_metrics
        },
        "pass": pass
    });

    let report_path = std::env::var("BSL_V2_COMPLETION_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-gate-{report_mode_suffix}.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for v2 completion gate report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize v2 completion gate report"),
    )
    .expect("failed to write v2 completion gate report");
    println!("v2_completion_gate_report={}", report_path.display());

    assert!(
        completion_p95 <= MAX_P95_MS,
        "acceptance gate failed: completion p95={}ms > {}ms",
        completion_p95,
        MAX_P95_MS
    );
    assert!(
        completion_p99 <= MAX_P99_MS,
        "acceptance gate failed: completion p99={}ms > {}ms",
        completion_p99,
        MAX_P99_MS
    );
    assert!(
        first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
        "acceptance gate failed: first-trigger success rate={:.4} < {:.4}",
        first_trigger_success_rate,
        MIN_FIRST_TRIGGER_SUCCESS_RATE
    );
    assert!(
        terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE,
        "acceptance gate failed: terminal-empty(fail_closed) rate={:.4} > {:.4}",
        terminal_empty_rate,
        MAX_TERMINAL_EMPTY_RATE
    );
    assert!(
        parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE,
        "acceptance gate failed: parity mismatch rate={:.4} > {:.4}",
        parity_mismatch_rate,
        MAX_PARITY_MISMATCH_RATE
    );
    assert!(
        parity_pairs_total >= PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
        "acceptance gate failed: parity pairs total={} < {}",
        parity_pairs_total,
        PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER
    );
    assert!(
        parity_evidence_verdict.is_ok(),
        "acceptance gate failed: parity evidence invalid: {}",
        parity_evidence_verdict
            .err()
            .unwrap_or_else(|| "unknown".to_string())
    );

    drain_task.abort();
}

#[test]
fn conf_big_root_resolution_prefers_explicit_override() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let default_root = workspace_root.join("examples").join("conf_big");
    let explicit_root = temp.path().join("external-conf-big");
    std::fs::create_dir_all(&default_root).expect("create default root");
    std::fs::create_dir_all(&explicit_root).expect("create explicit root");
    std::fs::write(default_root.join("Configuration.xml"), "<default />")
        .expect("write default config");
    std::fs::write(explicit_root.join("Configuration.xml"), "<explicit />")
        .expect("write explicit config");

    let resolved = conf_big_root_from_candidates(conf_big_candidate_roots_for_tests(
        &workspace_root,
        Some(explicit_root.clone()),
    ));

    assert_eq!(resolved.as_deref(), Some(explicit_root.as_path()));
}

#[test]
fn conf_big_root_resolution_falls_back_to_workspace_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let default_root = workspace_root.join("examples").join("conf_big");
    std::fs::create_dir_all(&default_root).expect("create default root");
    std::fs::write(default_root.join("Configuration.xml"), "<default />")
        .expect("write default config");

    let resolved =
        conf_big_root_from_candidates(conf_big_candidate_roots_for_tests(&workspace_root, None));

    assert_eq!(resolved.as_deref(), Some(default_root.as_path()));
}

#[test]
fn warm_completion_route_coverage_counts_missing_and_exact_routes() {
    let measured_samples = vec![
        serde_json::json!({
            "step": "measured_warm_completion_1",
            "trace": { "route": "head_hit" },
        }),
        serde_json::json!({
            "step": "measured_warm_completion_2",
            "trace": { "route": "exact_hit" },
        }),
        serde_json::json!({
            "step": "measured_warm_completion_3",
            "trace": { "route": null },
        }),
    ];

    assert_eq!(
        warm_completion_route_coverage(&measured_samples),
        WarmCompletionRouteCoverage {
            attributed_samples: 2,
            head_hit_samples: 1,
            exact_hit_samples: 1,
        }
    );
}

#[test]
fn assert_warm_completion_head_first_gate_rejects_missing_route_and_exact_hit() {
    let measured_samples = vec![
        serde_json::json!({
            "step": "measured_warm_completion_1",
            "trace": { "route": "head_hit" },
        }),
        serde_json::json!({
            "step": "measured_warm_completion_2",
            "trace": { "route": "exact_hit" },
        }),
        serde_json::json!({
            "step": "measured_warm_completion_3",
            "trace": { "route": null },
        }),
    ];

    let panic = std::panic::catch_unwind(|| {
        assert_warm_completion_head_first_gate(&measured_samples);
    })
    .expect_err("warm gate must reject missing route attribution and exact-hit regressions");
    let panic_text = if let Some(text) = panic.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = panic.downcast_ref::<&'static str>() {
        (*text).to_string()
    } else {
        "<non-string panic>".to_string()
    };
    assert!(
        panic_text.contains("route attribution"),
        "panic should mention missing route attribution, panic_text={panic_text}"
    );
}
