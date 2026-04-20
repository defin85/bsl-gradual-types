#[tokio::test]
async fn p7_newer_did_change_cooperatively_supersedes_obsolete_ready_snapshot_worker() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Возврат 2;\nКонецПроцедуры\n";
    const V3_FIXTURE: &str = "Процедура Тест()\n    Возврат 3;\nКонецПроцедуры\n";
    const DID_CHANGE_BLOCKING_PARSE_DELAY_MS: u64 = 1_200;

    let _env_lock = lock_test_env().await;
    let _blocking_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS",
        &DID_CHANGE_BLOCKING_PARSE_DELAY_MS.to_string(),
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

    let uri = Url::parse("file:///did_change_snapshot_supersede_fixture.bsl").expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");

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
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE.to_string(),
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

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let task_requested_version = {
                let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    task.target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .requested_version
                })
            };
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if task_requested_version == Some(2) && ready_version != Some(2) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange v2 must register an in-flight snapshot worker before supersession");

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
                            range: None,
                            range_length: None,
                            text: V3_FIXTURE.to_string(),
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

    tokio::time::timeout(Duration::from_secs(5), async {
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
    .expect("newest didChange revision must materialize its ready snapshot");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        )) >= 1,
        "newer didChange revision must still produce a didChange worker attempt, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted",
        )),
        0,
        "cooperative retarget path must not fall back to generic aborted attribution in this scenario, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change"
        )) > 0,
        "newest didChange revision must still materialize a ready snapshot after superseding the obsolete worker, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_same_file_did_change_burst_coalesces_obsolete_revisions_before_parse_starts() {
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Возврат 2;\nКонецПроцедуры\n";
    const V3_FIXTURE: &str = "Процедура Тест()\n    Возврат 3;\nКонецПроцедуры\n";
    const V4_FIXTURE: &str = "Процедура Тест()\n    Возврат 4;\nКонецПроцедуры\n";
    const DID_CHANGE_PARSE_DELAY_MS: u64 = 400;

    let _env_lock = lock_test_env().await;
    let _parse_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PARSE_DELAY_MS",
        &DID_CHANGE_PARSE_DELAY_MS.to_string(),
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

    let uri = Url::parse("file:///did_change_coalesced_before_parse_fixture.bsl").expect("uri");
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
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE.to_string(),
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

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if crate::server::language_server::did_change_inline_parse_delay_active_for_test() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange v2 must enter pre-parse async delay before burst retargeting");

    for (version, text) in [(3, V3_FIXTURE), (4, V4_FIXTURE)] {
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
                                version,
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
            .expect("burst didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didChange burst");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let ready_version = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.parse_snapshot.file_version);
            if ready_version == Some(4) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("latest burst revision must materialize its ready snapshot");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        )) < 3,
        "coalesced before-parse burst should need fewer logical worker starts than revisions, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse"
        )) > 0,
        "coalesced before-parse burst must export retargeted-before-parse attribution, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted"
        )),
        0,
        "before-parse coalescing must not regress into generic aborted attribution, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change"
        )) > 0,
        "latest burst revision must still materialize ready snapshot artifacts, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_parsed_did_change_revision_is_skipped_before_materialization_when_newer_target_arrives()
{
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

    const V1_FIXTURE: &str = "Процедура Тест()\n    Возврат 1;\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    Возврат 2;\nКонецПроцедуры\n";
    const V3_FIXTURE: &str = "Процедура Тест()\n    Возврат 3;\nКонецПроцедуры\n";
    const PRE_MATERIALIZATION_DELAY_MS: u64 = 500;

    let _env_lock = lock_test_env().await;
    let _pre_materialization_delay_guard = EnvVarGuard::set(
        "BSL_TEST_DID_CHANGE_PRE_MATERIALIZATION_DELAY_MS",
        &PRE_MATERIALIZATION_DELAY_MS.to_string(),
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

    let uri = Url::parse("file:///did_change_retargeted_before_materialization_fixture.bsl")
        .expect("uri");
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
                            range: None,
                            range_length: None,
                            text: V2_FIXTURE.to_string(),
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

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if crate::server::language_server::did_change_pre_materialization_delay_active_for_test(
            ) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange v2 must enter pre-materialization delay before retarget");

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
                            range: None,
                            range_length: None,
                            text: V3_FIXTURE.to_string(),
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after retarget");
    tokio::time::timeout(Duration::from_secs(5), async {
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
    .expect("newer target must materialize after skipping stale pre-materialization publish");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization"
        )) > 0,
        "parsed older revision must export retargeted-before-materialization attribution, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted"
        )),
        0,
        "stale pre-materialization skip must not fall back to generic aborted attribution, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p25_parsed_did_change_revision_is_retargeted_during_parse_when_newer_target_arrives() {
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

    fn build_large_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..1500 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _parse_progress_delay_guard =
        EnvVarGuard::set("BSL_TEST_PARSE_SNAPSHOT_PARSE_PROGRESS_DELAY_MS", "2");

    let v1_fixture = build_large_fixture("v1");
    let v2_fixture = build_large_fixture("v2");
    let v3_fixture = build_large_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_parse_fixture.bsl",
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
    .expect("opened fixture must materialize version 1 before during-parse retarget test");

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
                            range: None,
                            range_length: None,
                            text: v2_fixture.clone(),
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
            let phase = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .map(|control| control.phase.load(Ordering::SeqCst));
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(2)
                && phase
                    == Some(crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::Parsing as u8)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter parse_exec before retarget");

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
                            range: None,
                            range_length: None,
                            text: v3_fixture.clone(),
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
    .expect("newer target must materialize after during-parse retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "during-parse same-file retarget must export the dedicated lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "during-parse retarget test must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "during-parse retarget test must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "during-parse retarget test must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after during-parse retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p27_parsed_did_change_revision_is_retargeted_during_optional_cache_enrichment_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..64 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_optional_cache_enrichment_fixture.bsl",
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
    .expect("opened fixture must materialize version 1 before optional-enrichment retarget test");

    let _optional_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_OPTIONAL_CACHE_ENRICHMENT_DELAY_MS",
        "10000",
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
                            range: None,
                            range_length: None,
                            text: v2_fixture.clone(),
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
            let current_subphase = server
                .matching_background_parse_snapshot_task_control_v2(file_id, 2, Some(v2_hash))
                .await
                .and_then(|control| {
                    control
                        .phase_attribution_snapshot()
                        .current_parse_exec_subphase
                });
            if current_subphase
                == Some(
                    crate::server::ReadyParseSnapshotParseExecSubphaseV2::OptionalCacheEnrichment,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter optional cache enrichment before retarget");

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
                            range: None,
                            range_length: None,
                            text: v3_fixture.clone(),
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
    .expect("newer target must materialize after optional-enrichment retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "optional-enrichment retarget must still export the dedicated during-parse lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "optional-enrichment retarget must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "optional-enrichment retarget must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "optional-enrichment retarget must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after optional-enrichment retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p28_parsed_did_change_revision_is_retargeted_during_tree_cache_install_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..64 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_tree_cache_install_fixture.bsl",
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
    .expect("opened fixture must materialize version 1 before tree-cache-install retarget test");

    let _tree_cache_install_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_TREE_CACHE_INSTALL_DELAY_MS",
        "10000",
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
                            range: None,
                            range_length: None,
                            text: v2_fixture.clone(),
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
                        .current_core_build_checkpoint
                });
            if current_checkpoint
                == Some(crate::server::ReadyParseSnapshotCoreBuildCheckpointV2::TreeCacheInstall)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter tree-cache install before retarget");

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
                            range: None,
                            range_length: None,
                            text: v3_fixture.clone(),
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
    .expect("newer target must materialize after tree-cache-install retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "tree-cache-install retarget must still export the dedicated during-parse lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "tree-cache-install retarget must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "tree-cache-install retarget must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "tree-cache-install retarget must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after tree-cache-install retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p29_parsed_did_change_revision_is_retargeted_during_syntax_error_collection_when_newer_target_arrives(
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

    fn build_fixture(tag: &str) -> String {
        let mut text = String::from("Процедура Тест()\n");
        for idx in 0..64 {
            text.push_str(&format!("    Сообщить(\"{tag}-{idx}\");\n"));
        }
        text.push_str("КонецПроцедуры\n");
        text
    }

    let _env_lock = lock_test_env().await;
    let _debounce_guard =
        EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "0", true);

    let v1_fixture = build_fixture("v1");
    let v2_fixture = build_fixture("v2");
    let v3_fixture = build_fixture("v3");
    let v2_hash = *blake3::hash(v2_fixture.as_bytes()).as_bytes();

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        &v1_fixture,
        "file:///did_change_retargeted_during_syntax_error_collection_fixture.bsl",
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
    .expect("opened fixture must materialize version 1 before syntax-error-assembly retarget test");

    let _syntax_error_delay_guard = EnvVarGuard::set(
        "BSL_TEST_PARSE_SNAPSHOT_SYNTAX_ERROR_ASSEMBLY_DELAY_MS",
        "10000",
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
                            range: None,
                            range_length: None,
                            text: v2_fixture.clone(),
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
                == Some(
                    crate::server::ReadyParseSnapshotAssemblyCheckpointV2::SyntaxErrorCollection,
                )
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("version 2 didChange worker must enter syntax-error collection before retarget");

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
                            range: None,
                            range_length: None,
                            text: v3_fixture.clone(),
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
    .expect("newer target must materialize after syntax-error-assembly retarget");

    let final_metrics = server.coordinator.observability_metrics();
    let final_counters = final_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("final metrics.counters object");
    let retargeted_during_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse";
    let retargeted_before_parse_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse";
    let retargeted_before_materialization_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization";
    let aborted_key =
        "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted";

    let retargeted_during_parse_delta =
        read_u64_metric(final_counters.get(retargeted_during_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_during_parse_key)),
        );
    let retargeted_before_parse_delta =
        read_u64_metric(final_counters.get(retargeted_before_parse_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_parse_key)),
        );
    let retargeted_before_materialization_delta =
        read_u64_metric(final_counters.get(retargeted_before_materialization_key)).saturating_sub(
            read_u64_metric(baseline_counters.get(retargeted_before_materialization_key)),
        );
    let aborted_delta = read_u64_metric(final_counters.get(aborted_key))
        .saturating_sub(read_u64_metric(baseline_counters.get(aborted_key)));

    assert!(
        retargeted_during_parse_delta > 0,
        "syntax-error-assembly retarget must still export the dedicated during-parse lifecycle reason, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_parse_delta, 0,
        "syntax-error-assembly retarget must not regress into before-parse attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        retargeted_before_materialization_delta, 0,
        "syntax-error-assembly retarget must not regress into before-materialization attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );
    assert_eq!(
        aborted_delta, 0,
        "syntax-error-assembly retarget must not regress into generic aborted attribution, final_counters={final_counters:?}, baseline_counters={baseline_counters:?}"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let observed_text = analysis
        .file_text(file_id)
        .expect("file_text query")
        .expect("file text after syntax-error-assembly retarget");
    assert_eq!(observed_text.as_ref(), v3_fixture.as_str());

    drain_task.abort();
}

#[tokio::test]
async fn p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts(
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
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "4000");
    let _did_save_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_SAVE_BLOCKING_PARSE_DELAY_MS", "4000");
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

    let uri = Url::parse("file:///did_save_followup_apply_lag_fixture.bsl").expect("fixture");
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

    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file id after didOpen");
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
                            text: V2_FIXTURE.to_string(),
                        }],
                    })
                    .expect("DidChangeTextDocumentParams"),
                )
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
    .expect(
        "didChange must materialize ready parse snapshot before shadow-state fallback is disabled",
    );
    server
        .latest_document_shadow_state_v2
        .write()
        .await
        .remove(&file_id);
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

    tokio::time::timeout(Duration::from_millis(2500), async {
        loop {
            let params = published_rx
                .recv()
                .await
                .expect("publishDiagnostics channel must stay open");
            if params.uri != uri || params.version != Some(2) {
                continue;
            }
            break params;
        }
    })
    .await
    .expect("save_fastlane must still publish before stalled follow-up attribution");
    server
        .latest_ready_parse_snapshots_v2
        .write()
        .await
        .remove(&file_id);

    let trace = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_707, 8).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("diagnostics save timeline traces");
            if let Some(trace) = traces.iter().find(|trace| {
                trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                    && trace
                        .get("requested_version")
                        .and_then(|value| value.as_i64())
                        == Some(2)
                    && (trace
                        .get("followup_wait_reason")
                        .and_then(|value| value.as_str())
                        .is_some_and(|reason| reason != "pending_publish")
                        || trace.get("followup_publish").is_some())
            }) {
                break trace.clone();
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("matching diagnostics save timeline trace with explicit follow-up state");
    assert_eq!(
        trace
            .get("save_fastlane_outcome")
            .and_then(|value| value.as_str()),
        Some("published")
    );
    assert!(
        trace.get("followup_publish").is_none(),
        "apply-lagged heavy follow-up must not look published yet, trace={trace:?}"
    );
    let followup_wait_reason = trace
        .get("followup_wait_reason")
        .and_then(|value| value.as_str());
    assert_eq!(
        followup_wait_reason,
        Some("apply_lag"),
        "when shadow-state and ready-artifact follow-up paths are unavailable, in-flight heavy follow-up must expose apply_lag until writer-owned requested version is actually applied, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_semantic_path")
            .and_then(|value| value.as_str()),
        Some("generic_pipeline"),
        "apply-lag fallback must identify generic pipeline path before publish, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready"),
        "superseded path must retain zero-budget probe attribution before the bounded wait observes supersession, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("timeout")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("in_flight_same_version")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_relief_valve_outcome")
            .and_then(|value| value.as_str()),
        Some("skipped_apply_lag"),
        "apply-lag path must refuse temporary relief valve, trace={trace:?}"
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_relief_valve_budget_ms")
            .and_then(|value| value.as_u64()),
        Some(
            diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET
                .as_millis()
                .min(u64::MAX as u128) as u64
        )
    );
    assert!(
        trace
            .get("followup_apply_lag_ms")
            .and_then(|value| value.as_u64())
            .is_some_and(|value| value > 0),
        "apply-lag fallback trace must expose explicit followup_apply_lag_ms, trace={trace:?}"
    );
    assert!(
        trace.get("idle_heavy_outcome").is_none(),
        "idle_heavy must still be in-flight while apply lag is the primary blocker, trace={trace:?}"
    );
    assert!(
        trace.get("terminal_outcome").is_none(),
        "timeline must keep stalled heavy follow-up visible as active, trace={trace:?}"
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
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change"
        )) > 0,
        "apply-lag didSave path must retain worker-start attribution for the same-version didChange snapshot worker it waited on, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_not_ready"
        )) > 0,
        "apply-lag didSave path must export zero-budget probe miss counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_timeout"
        )) > 0,
        "apply-lag didSave path must export bounded-wait timeout counter, counters={counters:?}"
    );
    assert!(
        read_u64_metric(
            histograms
                .get("intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_timeout")
                .and_then(|value| value.get("count"))
        ) > 0,
        "apply-lag didSave path must export bounded-wait timeout latency histogram, histograms={histograms:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save"
        )) == 0,
        "apply-lag didSave path should still have no didSave ready snapshot materialization at capture time, counters={counters:?}"
    );
    assert!(
        read_u64_metric(
            counters
                .get("intellisense_v2_diagnostics_save_followup_wait_state_total_reason_apply_lag")
        ) > 0,
        "apply-lag didSave path must export apply_lag wait-state counter before generic pipeline starts, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(
            "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_apply_lag"
        )) > 0,
        "apply-lag path must export explicit relief-valve skip attribution, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_diagnostics_save_timeline_records_wait_probe_version_mismatch_on_stale_latest_version()
{
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

    let uri = Url::parse("file:///did_save_wait_probe_version_mismatch_trace_fixture.bsl")
        .expect("fixture");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(93),
        diagnostics_generation: 17,
        save_cycle_sequence: 5,
        requested_version: 14,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id: key.file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };

    let zero_probe = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &supersession_key,
        Duration::ZERO,
        Duration::ZERO,
        None,
        Some(key.diagnostics_generation),
        Some(key.requested_version),
    )
    .expect("zero-budget probe outcome");
    let wait_probe = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &supersession_key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        None,
        Some(key.diagnostics_generation),
        Some(key.requested_version + 1),
    )
    .expect("version-mismatch probe outcome");

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
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some(zero_probe.as_str()),
        Some(wait_probe.as_str()),
        Some("in_flight_same_version"),
        Some(false),
        None,
    );
    server.record_diagnostics_save_timeline_profile_disposition(
        &uri,
        key,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        bsl_runtime::application::DiagnosticsDisposition::SupersededVersion,
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_718, 12).await;
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
                    == Some(key.requested_version as i64)
        })
        .expect("version-mismatch diagnostics save trace");
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("version_mismatch")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("in_flight_same_version")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_diagnostics_save_timeline_records_wait_probe_superseded_when_newer_change_arrives() {
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
        Url::parse("file:///did_save_wait_probe_superseded_trace_fixture.bsl").expect("fixture");
    let key = crate::server::DiagnosticsSaveTimelineCycleKey {
        file_id: bsl_analysis_v2::FileId(94),
        diagnostics_generation: 19,
        save_cycle_sequence: 6,
        requested_version: 18,
    };
    let supersession_key = crate::server::DiagnosticsSupersessionKeyV2 {
        file_id: key.file_id,
        profile: bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        diagnostics_generation: key.diagnostics_generation,
        save_cycle_sequence: Some(key.save_cycle_sequence),
        requested_version: key.requested_version,
    };

    let zero_probe = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &supersession_key,
        Duration::ZERO,
        Duration::ZERO,
        None,
        Some(key.diagnostics_generation),
        Some(key.requested_version),
    )
    .expect("zero-budget probe outcome");
    let wait_probe = BslLanguageServer::ready_parse_snapshot_probe_wait_decision_v2(
        &supersession_key,
        diagnostics_runtime::SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
        Duration::from_millis(5),
        Some(crate::server::DiagnosticsCancellationReasonV2::SupersededVersion),
        Some(key.diagnostics_generation),
        Some(key.requested_version),
    )
    .expect("superseded probe outcome");

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
    server.record_diagnostics_save_timeline_followup_probe_state(
        &uri,
        key,
        Some(zero_probe.as_str()),
        Some(wait_probe.as_str()),
        Some("absent"),
        Some(false),
        None,
    );
    server.record_diagnostics_save_timeline_profile_disposition(
        &uri,
        key,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy,
        bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
    );

    let timeline = lsp_get_diagnostics_save_timeline(&mut service, 50_719, 12).await;
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
                    == Some(key.requested_version as i64)
        })
        .expect("superseded diagnostics save trace");
    assert_eq!(
        trace
            .get("followup_ready_snapshot_zero_probe")
            .and_then(|value| value.as_str()),
        Some("not_ready")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_wait_probe")
            .and_then(|value| value.as_str()),
        Some("superseded")
    );
    assert_eq!(
        trace
            .get("followup_ready_snapshot_task_state")
            .and_then(|value| value.as_str()),
        Some("absent")
    );
    assert_eq!(
        trace
            .get("followup_shadow_state_available")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        trace
            .get("idle_heavy_outcome")
            .and_then(|value| value.as_str()),
        Some("superseded_generation")
    );
    assert_eq!(
        trace
            .get("terminal_outcome")
            .and_then(|value| value.as_str()),
        Some("superseded_generation")
    );

    drain_task.abort();
}
