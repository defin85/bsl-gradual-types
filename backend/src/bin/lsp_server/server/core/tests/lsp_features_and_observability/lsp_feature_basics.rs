#[tokio::test]
async fn p9a_formatting_disabled_does_not_advertise_capability_and_returns_null() {
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
    let response = response.expect("initialize should return a response");

    let response_value = serde_json::to_value(&response).expect("serialize initialize response");
    let capabilities = response_value
        .get("result")
        .and_then(|v| v.get("capabilities"))
        .expect("initialize capabilities");

    let execute_commands = capabilities
        .get("executeCommandProvider")
        .and_then(|v| v.get("commands"))
        .and_then(|v| v.as_array())
        .expect("executeCommandProvider.commands");
    assert!(
        execute_commands
            .iter()
            .any(|command| command.as_str() == Some("bsl.getAllTypes")),
        "initialize must advertise bsl.getAllTypes execute command, got {execute_commands:?}"
    );

    match capabilities.get("documentFormattingProvider") {
        None => {}
        Some(v) => assert!(
            v.is_null(),
            "documentFormattingProvider must be absent/null"
        ),
    }
    match capabilities.get("documentRangeFormattingProvider") {
        None => {}
        Some(v) => assert!(
            v.is_null(),
            "documentRangeFormattingProvider must be absent/null"
        ),
    }

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

    let uri = Url::parse("file:///test_p9a_formatting_disabled.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура Тест()\nКонецПроцедуры\n".to_string(),
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

    let formatting_params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let formatting_req = Request::build("textDocument/formatting")
        .id(2)
        .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
        .finish();
    let formatting_response = service
        .ready()
        .await
        .unwrap()
        .call(formatting_req)
        .await
        .expect("formatting request");
    let formatting_response = formatting_response.expect("formatting should return a response");

    let response_value =
        serde_json::to_value(&formatting_response).expect("serialize formatting response");
    match response_value.get("error") {
        None => {}
        Some(v) => assert!(v.is_null(), "formatting must not return an error"),
    }
    let result = response_value
        .get("result")
        .cloned()
        .expect("formatting result field");
    assert!(
        result.is_null(),
        "disabled formatting should return null edits"
    );

    let range_formatting_params = DocumentRangeFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        options: FormattingOptions::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let range_req = Request::build("textDocument/rangeFormatting")
        .id(3)
        .params(
            serde_json::to_value(range_formatting_params).expect("DocumentRangeFormattingParams"),
        )
        .finish();
    let range_response = service
        .ready()
        .await
        .unwrap()
        .call(range_req)
        .await
        .expect("rangeFormatting request");
    let range_response = range_response.expect("rangeFormatting should return a response");

    let range_value =
        serde_json::to_value(&range_response).expect("serialize rangeFormatting response");
    match range_value.get("error") {
        None => {}
        Some(v) => assert!(v.is_null(), "rangeFormatting must not return an error"),
    }
    let range_result = range_value
        .get("result")
        .cloned()
        .expect("rangeFormatting result field");
    assert!(
        range_result.is_null(),
        "disabled rangeFormatting should return null edits"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p9_formatting_reindents_and_trims_when_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    // LSP initialize handshake is required, otherwise client notifications are suppressed.
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

    // Enable formatting through didChangeConfiguration (section `bsl`).
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
                    "enabled": true,
                    "indentSize": 4
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p9_formatting.bsl").expect("test uri");
    let text = "Процедура Тест()\nЕсли Истина Тогда  \nСообщить(1);\nИначе\nСообщить(2);   \nКонецЕсли;\nКонецПроцедуры\n";

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

    let formatting_params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let formatting_req = Request::build("textDocument/formatting")
        .id(2)
        .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
        .finish();
    let formatting_response = service
        .ready()
        .await
        .unwrap()
        .call(formatting_req)
        .await
        .expect("formatting request");
    let formatting_response = formatting_response.expect("formatting should return a response");

    let response_value =
        serde_json::to_value(&formatting_response).expect("serialize formatting response");
    let edits_value = response_value
        .get("result")
        .cloned()
        .expect("formatting result field");
    let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_value).expect("parse edits");
    let edits = edits.expect("edits present");
    assert!(!edits.is_empty(), "formatting must return edits");

    // Apply per-line edits (formatter emits full-line replacements).
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    for edit in edits {
        let line = edit.range.start.line as usize;
        lines[line] = edit.new_text;
    }
    let formatted = lines.join("\n");

    let expected = "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(1);\n    Иначе\n        Сообщить(2);\n    КонецЕсли;\nКонецПроцедуры\n";
    assert_eq!(formatted, expected);

    drain_task.abort();
}

#[tokio::test]
async fn p10_range_formatting_only_updates_selected_lines() {
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
                    "enabled": true,
                    "indentSize": 4
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p10_range_formatting.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    Сообщить(\"a\");\n",
        "Если Истина Тогда\n",
        "Сообщить(1);\n",
        "КонецЕсли;\n",
        "    Сообщить(\"b\");\n",
        "КонецПроцедуры\n",
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

    let range_formatting_params = DocumentRangeFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range {
            start: Position {
                line: 2,
                character: 0,
            },
            end: Position {
                line: 5,
                character: 0,
            },
        },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let range_req = Request::build("textDocument/rangeFormatting")
        .id(2)
        .params(
            serde_json::to_value(range_formatting_params).expect("DocumentRangeFormattingParams"),
        )
        .finish();

    let response_a = service
        .ready()
        .await
        .unwrap()
        .call(range_req)
        .await
        .expect("rangeFormatting request");
    let response_a = response_a.expect("rangeFormatting should return a response");

    let response_value =
        serde_json::to_value(&response_a).expect("serialize rangeFormatting response");
    let edits_value = response_value
        .get("result")
        .cloned()
        .expect("rangeFormatting result field");
    let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_value).expect("parse edits");
    let edits = edits.expect("edits present");

    assert_eq!(edits.len(), 3, "expected 3 line edits inside the range");
    for edit in &edits {
        assert!(
            (2..=4).contains(&edit.range.start.line),
            "unexpected edit line {:?}",
            edit.range.start.line
        );
    }
    let projected_a: Vec<(u32, String)> = edits
        .iter()
        .map(|edit| (edit.range.start.line, edit.new_text.clone()))
        .collect();

    // Apply per-line edits.
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    for edit in edits {
        let line = edit.range.start.line as usize;
        lines[line] = edit.new_text;
    }
    let formatted = lines.join("\n");

    let expected = concat!(
        "Процедура Тест()\n",
        "    Сообщить(\"a\");\n",
        "    Если Истина Тогда\n",
        "        Сообщить(1);\n",
        "    КонецЕсли;\n",
        "    Сообщить(\"b\");\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(formatted, expected);

    // Determinism: second request returns identical edits.
    let range_req_2 = Request::build("textDocument/rangeFormatting")
        .id(3)
        .params(
            serde_json::to_value(DocumentRangeFormattingParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 0,
                    },
                },
                options: FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    ..Default::default()
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .expect("DocumentRangeFormattingParams"),
        )
        .finish();

    let response_b = service
        .ready()
        .await
        .unwrap()
        .call(range_req_2)
        .await
        .expect("rangeFormatting request (2)");
    let response_b = response_b.expect("rangeFormatting (2) should return a response");

    let value_b = serde_json::to_value(&response_b).expect("serialize response");
    let edits_b_value = value_b.get("result").cloned().expect("result field");
    let edits_b: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_b_value).expect("parse edits");
    let edits_b = edits_b.expect("edits present");
    let projected_b: Vec<(u32, String)> = edits_b
        .iter()
        .map(|edit| (edit.range.start.line, edit.new_text.clone()))
        .collect();
    assert_eq!(
        projected_b, projected_a,
        "range formatting must be deterministic"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p11_document_symbol_groups_routines_by_region() {
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

    let uri = Url::parse("file:///test_p11_document_symbol.bsl").expect("test uri");
    let text = concat!(
        "#Область Public\n",
        "Процедура Inside() Экспорт\n",
        "КонецПроцедуры\n",
        "#КонецОбласти\n",
        "Функция Outside() Экспорт\n",
        "КонецФункции\n",
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

    let parsed_a = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(response) = lsp_document_symbol_with_request(&mut service, 2, &uri).await {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("documentSymbol response must arrive");
    let result_a_value = serde_json::to_value(&parsed_a).expect("serialize first result");

    let DocumentSymbolResponse::Nested(top_level) = parsed_a else {
        panic!("expected nested document symbols");
    };

    let region = top_level
        .iter()
        .find(|sym| sym.name == "Public")
        .expect("expected region Public");
    assert_eq!(region.kind, SymbolKind::NAMESPACE);

    let children = region.children.as_ref().expect("region must have children");
    let inside = children
        .iter()
        .find(|sym| sym.name == "Inside")
        .expect("expected Inside");
    assert_eq!(inside.kind, SymbolKind::METHOD);
    assert_eq!(inside.detail.as_deref(), Some("export"));
    assert_eq!(inside.range.start.line, 1);
    assert_eq!(inside.selection_range.start.line, 1);
    assert_eq!(inside.selection_range.start.character, 10);
    assert_eq!(inside.selection_range.end.character, 16);

    let outside = top_level
        .iter()
        .find(|sym| sym.name == "Outside")
        .expect("expected Outside");
    assert_eq!(outside.kind, SymbolKind::FUNCTION);
    assert_eq!(outside.detail.as_deref(), Some("export"));
    assert_eq!(outside.selection_range.start.line, 4);
    assert_eq!(outside.selection_range.start.character, 8);
    assert_eq!(outside.selection_range.end.character, 15);

    // Determinism: second request returns identical JSON result.
    let response_b = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(response) = lsp_document_symbol_with_request(&mut service, 3, &uri).await {
                break response;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("second documentSymbol response must arrive");
    let result_b_value = serde_json::to_value(&response_b).expect("serialize second result");
    assert_eq!(
        result_a_value, result_b_value,
        "documentSymbol must be deterministic"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p12_workspace_symbol_searches_open_documents() {
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

    let uri_a = Url::parse("file:///test_p12_a.bsl").expect("test uri a");
    let uri_b = Url::parse("file:///test_p12_b.bsl").expect("test uri b");

    let did_open_a = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri_a.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура FooOne() Экспорт\nКонецПроцедуры\n".to_string(),
        },
    };
    let did_open_b = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri_b.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Функция FooTwo() Экспорт\nКонецФункции\n".to_string(),
        },
    };

    for did_open in [did_open_a, did_open_b] {
        let req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let resp = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("didOpen notification");
        assert!(resp.is_none(), "didOpen is a notification");
    }

    let params = WorkspaceSymbolParams {
        query: "Foo".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("workspace/symbol")
        .id(2)
        .params(serde_json::to_value(params).expect("WorkspaceSymbolParams"))
        .finish();

    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("workspace/symbol request");
    let response = response.expect("workspace/symbol should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<SymbolInformation>> =
        serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    assert!(
        parsed
            .iter()
            .any(|sym| sym.name == "FooOne" && sym.location.uri == uri_a),
        "expected FooOne in uri_a, got {:?}",
        parsed
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        parsed
            .iter()
            .any(|sym| sym.name == "FooTwo" && sym.location.uri == uri_b),
        "expected FooTwo in uri_b, got {:?}",
        parsed
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );

    let empty_query_params = WorkspaceSymbolParams {
        query: String::new(),
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let empty_query_req = Request::build("workspace/symbol")
        .id(3)
        .params(serde_json::to_value(empty_query_params).expect("WorkspaceSymbolParams"))
        .finish();

    let empty_query_response = service
        .ready()
        .await
        .unwrap()
        .call(empty_query_req)
        .await
        .expect("workspace/symbol empty-query request");
    let empty_query_response =
        empty_query_response.expect("workspace/symbol empty-query should return a response");

    let empty_query_value = serde_json::to_value(&empty_query_response).expect("serialize response");
    let empty_query_result_value = empty_query_value.get("result").cloned().expect("result field");
    let empty_query_symbols: Option<Vec<SymbolInformation>> =
        serde_json::from_value(empty_query_result_value).expect("parse empty-query result");
    let empty_query_symbols = empty_query_symbols.expect("empty-query result present");

    assert!(
        empty_query_symbols
            .iter()
            .any(|sym| sym.name == "FooOne" && sym.location.uri == uri_a),
        "expected FooOne for empty query, got {:?}",
        empty_query_symbols
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        empty_query_symbols
            .iter()
            .any(|sym| sym.name == "FooTwo" && sym.location.uri == uri_b),
        "expected FooTwo for empty query, got {:?}",
        empty_query_symbols
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );

    drain_task.abort();
}

#[tokio::test]
async fn p13_unclosed_region_is_closed_at_eof() {
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

    let uri = Url::parse("file:///test_p13_unclosed_region.bsl").expect("test uri");
    let text = concat!(
        "#Область Unclosed\n",
        "Процедура Inside() Экспорт\n",
        "КонецПроцедуры\n",
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

    let req = Request::build("textDocument/documentSymbol")
        .id(2)
        .params(
            serde_json::to_value(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("DocumentSymbolParams"),
        )
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("documentSymbol request");
    let response = response.expect("documentSymbol should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<DocumentSymbolResponse> =
        serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    let DocumentSymbolResponse::Nested(top_level) = parsed else {
        panic!("expected nested document symbols");
    };

    let region = top_level
        .iter()
        .find(|sym| sym.name == "Unclosed")
        .expect("expected region Unclosed");
    assert_eq!(region.kind, SymbolKind::NAMESPACE);
    assert_eq!(
        region.range.end,
        Position {
            line: 3,
            character: 0,
        },
        "unclosed region should be closed at EOF"
    );

    let children = region.children.as_ref().expect("region must have children");
    assert!(
        children.iter().any(|sym| sym.name == "Inside"),
        "expected Inside inside Unclosed region"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p14_references_returns_local_var_locations_and_respects_include_declaration() {
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

    let uri = Url::parse("file:///test_p14_references.bsl").expect("test uri");
    let text = concat!(
        "Процедура T()\n",
        "    Перем X;\n",
        "    X = 1;\n",
        "    Сообщить(X);\n",
        "КонецПроцедуры\n",
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

    let params_with_decl = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };

    let req_with_decl = Request::build("textDocument/references")
        .id(2)
        .params(serde_json::to_value(params_with_decl).expect("ReferenceParams"))
        .finish();
    let response_with_decl = service
        .ready()
        .await
        .unwrap()
        .call(req_with_decl)
        .await
        .expect("references request");
    let response_with_decl = response_with_decl.expect("references should return a response");

    let value = serde_json::to_value(&response_with_decl).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    assert_eq!(parsed.len(), 3, "expected declaration + 2 usages");
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 1,
                    character: 10
                },
                end: Position {
                    line: 1,
                    character: 11
                }
            }),
        "expected declaration location for X"
    );
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 2,
                    character: 4
                },
                end: Position {
                    line: 2,
                    character: 5
                }
            }),
        "expected assignment target usage for X"
    );
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 3,
                    character: 13
                },
                end: Position {
                    line: 3,
                    character: 14
                }
            }),
        "expected call argument usage for X"
    );

    let params_no_decl = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: false,
        },
    };

    let req_no_decl = Request::build("textDocument/references")
        .id(3)
        .params(serde_json::to_value(params_no_decl).expect("ReferenceParams"))
        .finish();
    let response_no_decl = service
        .ready()
        .await
        .unwrap()
        .call(req_no_decl)
        .await
        .expect("references request (no decl)");
    let response_no_decl = response_no_decl.expect("references (no decl) should return a response");
    let value = serde_json::to_value(&response_no_decl).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");
    assert_eq!(parsed.len(), 2, "expected 2 usages without declaration");

    drain_task.abort();
}

#[tokio::test]
async fn p15_rename_updates_only_target_symbol_and_prepare_rename_is_supported() {
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

    let uri = Url::parse("file:///test_p15_rename.bsl").expect("test uri");
    let text = concat!(
        "Процедура T()\n",
        "    Перем X;\n",
        "    Перем XX;\n",
        "    X = 1;\n",
        "    XX = 2;\n",
        "    Сообщить(X);\n",
        "    Сообщить(XX);\n",
        "КонецПроцедуры\n",
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

    let prepare_req = Request::build("textDocument/prepareRename")
        .id(2)
        .params(
            serde_json::to_value(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 5,
                    character: 13,
                },
            })
            .expect("TextDocumentPositionParams"),
        )
        .finish();
    let prepare_resp = service
        .ready()
        .await
        .unwrap()
        .call(prepare_req)
        .await
        .expect("prepareRename request");
    let prepare_resp = prepare_resp.expect("prepareRename should return a response");
    let value = serde_json::to_value(&prepare_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<PrepareRenameResponse> =
        serde_json::from_value(result_value).expect("parse prepareRename");
    let parsed = parsed.expect("result present");
    match parsed {
        PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => {
            assert_eq!(placeholder, "X");
            assert_eq!(
                range,
                Range {
                    start: Position {
                        line: 5,
                        character: 13
                    },
                    end: Position {
                        line: 5,
                        character: 14
                    }
                }
            );
        }
        other => panic!("unexpected prepareRename response: {:?}", other),
    }

    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 5,
                character: 13,
            },
        },
        new_name: "Y".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let rename_req = Request::build("textDocument/rename")
        .id(3)
        .params(serde_json::to_value(rename_params).expect("RenameParams"))
        .finish();

    let rename_resp = service
        .ready()
        .await
        .unwrap()
        .call(rename_req)
        .await
        .expect("rename request");
    let rename_resp = rename_resp.expect("rename should return a response");

    let value = serde_json::to_value(&rename_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<WorkspaceEdit> =
        serde_json::from_value(result_value).expect("parse workspace edit");
    let parsed = parsed.expect("result present");
    let changes = parsed.changes.expect("changes present");
    let edits = changes.get(&uri).expect("edits for uri");
    assert_eq!(edits.len(), 3, "expected declaration + 2 usages for X");
    assert!(
        edits.iter().all(|e| e.new_text == "Y"),
        "all edits must rename to Y"
    );
    assert!(
        edits.iter().all(|e| e.range.start.line != 2),
        "must not touch XX declaration line"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p16_references_returns_routine_declaration_and_calls() {
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

    let uri = Url::parse("file:///test_p16_routine_references.bsl").expect("test uri");
    let text = concat!(
        "Процедура Foo() Экспорт\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Bar()\n",
        "    Foo();\n",
        "    Foo();\n",
        "КонецПроцедуры\n",
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

    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 4,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };

    let req = Request::build("textDocument/references")
        .id(2)
        .params(serde_json::to_value(params).expect("ReferenceParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("references request");
    let response = response.expect("references should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");
    assert_eq!(parsed.len(), 3, "expected declaration + 2 call sites");

    drain_task.abort();
}

#[tokio::test]
async fn p17_rename_routine_updates_declaration_and_calls_only() {
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

    let uri = Url::parse("file:///test_p17_routine_rename.bsl").expect("test uri");
    let text = concat!(
        "Процедура Foo() Экспорт\n",
        "КонецПроцедуры\n",
        "Процедура FooX() Экспорт\n",
        "КонецПроцедуры\n",
        "Процедура Bar()\n",
        "    Foo();\n",
        "    FooX();\n",
        "    Foo();\n",
        "КонецПроцедуры\n",
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

    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 5,
                character: 4,
            },
        },
        new_name: "Baz".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let rename_req = Request::build("textDocument/rename")
        .id(2)
        .params(serde_json::to_value(rename_params).expect("RenameParams"))
        .finish();

    let rename_resp = service
        .ready()
        .await
        .unwrap()
        .call(rename_req)
        .await
        .expect("rename request");
    let rename_resp = rename_resp.expect("rename should return a response");

    let value = serde_json::to_value(&rename_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<WorkspaceEdit> =
        serde_json::from_value(result_value).expect("parse workspace edit");
    let parsed = parsed.expect("result present");
    let changes = parsed.changes.expect("changes present");
    let edits = changes.get(&uri).expect("edits for uri");

    assert!(
        edits.iter().all(|e| e.new_text == "Baz"),
        "all edits must rename to Baz"
    );
    assert_eq!(
        edits.len(),
        3,
        "expected declaration + 2 call sites for Foo"
    );
    assert!(
        edits.iter().all(|e| e.range.start.line != 6),
        "must not touch FooX() call"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p18_capabilities_gate_inlay_hints_and_code_actions() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": false
        })),
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
        .expect("initialize request")
        .expect("initialize response");

    let response_value = serde_json::to_value(&response).expect("serialize initialize response");
    let caps = response_value
        .get("result")
        .and_then(|v| v.get("capabilities"))
        .expect("initialize result.capabilities");

    assert!(
        caps.get("inlayHintProvider").is_some(),
        "inlayHintProvider must be present when enableTypeHints=true"
    );
    let code_actions = caps.get("codeActionProvider");
    assert!(
        code_actions.is_none() || code_actions.is_some_and(|v| v.is_null()),
        "codeActionProvider must be absent/null when enableCodeActions=false"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p19_inlay_hints_returns_type_hints_when_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": false
        })),
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
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": false
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p19_inlay_hints.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
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

    let params = InlayHintParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(0, 0), Position::new(10, 0)),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let req = Request::build("textDocument/inlayHint")
        .id(2)
        .params(serde_json::to_value(params).expect("InlayHintParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("inlayHint request")
        .expect("inlayHint response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let hints: Option<Vec<InlayHint>> = serde_json::from_value(result_value).expect("parse hints");
    let hints = hints.expect("hints present");

    assert!(!hints.is_empty(), "expected at least one hint");
    assert!(
        hints.iter().any(
            |hint| matches!(&hint.label, InlayHintLabel::String(text) if text.contains(": Число"))
        ),
        "expected ': Число' hint"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p20_code_actions_return_quickfix_add_type_annotation() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": true
        })),
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
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": true
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p20_code_actions.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
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

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(2, 0), Position::new(2, 5)),
        context: CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("textDocument/codeAction")
        .id(2)
        .params(serde_json::to_value(params).expect("CodeActionParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("codeAction request")
        .expect("codeAction response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let actions: Option<Vec<CodeActionOrCommand>> =
        serde_json::from_value(result_value).expect("parse actions");
    let actions = actions.expect("actions present");

    assert!(
        actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::QUICKFIX))),
        "expected at least one quickfix action"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p21_code_actions_return_extract_refactor_on_selection() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": true
        })),
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
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": true
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p21_code_actions.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
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

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(2, 4), Position::new(2, 5)),
        context: CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("textDocument/codeAction")
        .id(2)
        .params(serde_json::to_value(params).expect("CodeActionParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("codeAction request")
        .expect("codeAction response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let actions: Option<Vec<CodeActionOrCommand>> =
        serde_json::from_value(result_value).expect("parse actions");
    let actions = actions.expect("actions present");

    assert!(
        actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::REFACTOR_EXTRACT))),
        "expected refactor.extract action"
    );

    drain_task.abort();
}
