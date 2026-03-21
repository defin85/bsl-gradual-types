use super::*;
use serde_json::json;

#[tokio::test]
async fn current_request_id_is_none_outside_scope() {
    assert_eq!(current_request_id(), None);
}

#[tokio::test]
async fn current_request_received_at_ms_is_none_outside_scope() {
    assert_eq!(current_request_received_at_ms(), None);
}

#[tokio::test]
async fn current_request_service_scope_entered_at_ms_is_none_outside_scope() {
    assert_eq!(current_request_service_scope_entered_at_ms(), None);
}

#[tokio::test]
async fn current_request_service_future_created_at_ms_is_none_outside_scope() {
    assert_eq!(current_request_service_future_created_at_ms(), None);
}

#[tokio::test]
async fn with_request_context_exposes_context_inside_scope() {
    let scoped = with_request_context(
        Some("42".to_string()),
        Some(1_700_000_000_123),
        Some(1_700_000_000_124),
        Some(1_700_000_000_125),
        async {
            (
                current_request_id(),
                current_request_received_at_ms(),
                current_request_service_future_created_at_ms(),
                current_request_service_scope_entered_at_ms(),
            )
        },
    )
    .await;
    assert_eq!(scoped.0, Some("42".to_string()));
    assert_eq!(scoped.1, Some(1_700_000_000_123));
    assert_eq!(scoped.2, Some(1_700_000_000_124));
    assert_eq!(scoped.3, Some(1_700_000_000_125));
}

#[tokio::test]
async fn request_context_service_sets_jsonrpc_numeric_id() {
    #[derive(Clone, Debug, Default)]
    struct CaptureService;

    impl Service<Request> for CaptureService {
        type Response = (Option<String>, Option<u64>, Option<u64>, Option<u64>);
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            Box::pin(async move {
                Ok((
                    current_request_id(),
                    current_request_received_at_ms(),
                    current_request_service_future_created_at_ms(),
                    current_request_service_scope_entered_at_ms(),
                ))
            })
        }
    }

    let mut service = RequestContextService::new(CaptureService);
    let request = Request::build("workspace/symbol").id(9_i64).finish();
    let captured = service.call(request).await.expect("service call");
    assert_eq!(captured.0, Some("9".to_string()));
    assert!(
        captured.1.is_some(),
        "request receive timestamp must be scoped"
    );
    assert!(
        captured.2.is_some(),
        "service future creation timestamp must be scoped"
    );
    assert!(
        captured.3.is_some(),
        "service-scope enter timestamp must be scoped"
    );
    assert!(
        captured.2.unwrap() >= captured.1.unwrap(),
        "service future creation timestamp must not be earlier than request receive timestamp"
    );
    assert!(
        captured.3.unwrap() >= captured.2.unwrap(),
        "service-scope enter timestamp must not be earlier than request receive timestamp"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_context_service_does_not_propagate_request_id_to_spawned_handler() {
    #[derive(Clone, Debug, Default)]
    struct SpawnedCaptureService;

    impl Service<Request> for SpawnedCaptureService {
        type Response = (Option<String>, Option<u64>, Option<u64>, Option<u64>);
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            Box::pin(async move {
                let captured = tokio::spawn(async move {
                    (
                        current_request_id(),
                        current_request_received_at_ms(),
                        current_request_service_future_created_at_ms(),
                        current_request_service_scope_entered_at_ms(),
                    )
                })
                .await
                .expect("spawned capture join");
                Ok(captured)
            })
        }
    }

    let mut service = RequestContextService::new(SpawnedCaptureService);
    let request = Request::build("workspace/symbol").id(77_i64).finish();
    let captured = service.call(request).await.expect("service call");
    assert_eq!(captured.0, None);
    assert_eq!(captured.1, None);
    assert_eq!(captured.2, None);
    assert_eq!(captured.3, None);
}

#[tokio::test]
async fn request_context_service_records_completion_id_for_position_lookup() {
    #[derive(Clone, Debug)]
    struct TakeCaptureService {
        uri: Url,
        position: Position,
    }

    impl Service<Request> for TakeCaptureService {
        type Response = Option<String>;
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            let uri = self.uri.clone();
            let position = self.position;
            Box::pin(async move { Ok(take_completion_request_id(&uri, position)) })
        }
    }

    let uri = Url::parse("file:///request_context_service_record.bsl").expect("url");
    let position = Position::new(4, 11);
    let mut service = RequestContextService::new(TakeCaptureService {
        uri: uri.clone(),
        position,
    });
    let completion_params = CompletionParams {
        text_document_position: tower_lsp::lsp_types::TextDocumentPositionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position,
        },
        work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
        partial_result_params: tower_lsp::lsp_types::PartialResultParams::default(),
        context: None,
    };
    let request = Request::build("textDocument/completion")
        .id("req-service")
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();

    let captured = service.call(request).await.expect("service call");
    assert_eq!(captured, Some("req-service".to_string()));
}

#[tokio::test]
async fn request_context_service_records_completion_context_for_position_lookup() {
    #[derive(Clone, Debug)]
    struct TakeCaptureService {
        uri: Url,
        position: Position,
    }

    impl Service<Request> for TakeCaptureService {
        type Response = Option<PendingCompletionRequestContext>;
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            let uri = self.uri.clone();
            let position = self.position;
            Box::pin(async move { Ok(take_completion_request_context(&uri, position)) })
        }
    }

    let uri = Url::parse("file:///request_context_service_record_context.bsl").expect("url");
    let position = Position::new(5, 3);
    let mut service = RequestContextService::new(TakeCaptureService {
        uri: uri.clone(),
        position,
    });
    let completion_params = CompletionParams {
        text_document_position: tower_lsp::lsp_types::TextDocumentPositionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position,
        },
        work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
        partial_result_params: tower_lsp::lsp_types::PartialResultParams::default(),
        context: None,
    };
    let request = Request::build("textDocument/completion")
        .id("req-context")
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();

    let captured = service.call(request).await.expect("service call");
    let captured = captured.expect("captured request context");
    assert_eq!(captured.request_id, "req-context");
    assert!(captured.request_received_at_ms.is_some());
    assert!(captured.service_future_created_at_ms.is_some());
    assert!(captured.service_scope_entered_at_ms.is_some());
    assert!(
        captured.service_future_created_at_ms.unwrap() >= captured.request_received_at_ms.unwrap()
    );
    assert!(
        captured.service_scope_entered_at_ms.unwrap()
            >= captured.service_future_created_at_ms.unwrap()
    );
}

#[test]
fn completion_request_id_is_recorded_and_taken_by_position_key() {
    let uri = Url::parse("file:///request_context_completion.bsl").expect("url");
    let request_id = "req-42";
    let request = Request::build("textDocument/completion")
        .id(request_id)
        .params(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 3, "character": 7 },
        }))
        .finish();
    record_pending_completion_request_id(&request, request_id, None);

    let taken = take_completion_request_id(
        &Url::parse("file:///request_context_completion.bsl").expect("url"),
        Position::new(3, 7),
    );
    assert_eq!(taken, Some(request_id.to_string()));
    assert_eq!(
        take_completion_request_id(
            &Url::parse("file:///request_context_completion.bsl").expect("url"),
            Position::new(3, 7),
        ),
        None
    );
}

#[test]
fn overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order() {
    let uri = Url::parse("file:///request_context_overlap.bsl").expect("url");
    let position = Position::new(8, 4);
    let first_request_id = "req-1";
    let second_request_id = "req-2";
    let first_request = Request::build("textDocument/completion")
        .id(first_request_id)
        .params(json!({
            "textDocument": { "uri": uri },
            "position": { "line": position.line, "character": position.character },
        }))
        .finish();
    let second_request = Request::build("textDocument/completion")
        .id(second_request_id)
        .params(json!({
            "textDocument": { "uri": uri },
            "position": { "line": position.line, "character": position.character },
        }))
        .finish();

    record_pending_completion_request_id(&first_request, first_request_id, Some(1_700_000_000_010));
    record_pending_completion_service_future_created_at_ms(first_request_id, 1_700_000_000_011);
    record_pending_completion_service_scope_entered_at_ms(first_request_id, 1_700_000_000_012);
    record_pending_completion_request_id(
        &second_request,
        second_request_id,
        Some(1_700_000_000_020),
    );
    record_pending_completion_service_future_created_at_ms(second_request_id, 1_700_000_000_020);
    record_pending_completion_service_scope_entered_at_ms(second_request_id, 1_700_000_000_021);

    let second = take_completion_request_context_by_request_id(second_request_id)
        .expect("second request should be taken by request id");
    assert_eq!(second.request_id, second_request_id);
    assert_eq!(second.request_received_at_ms, Some(1_700_000_000_020));
    assert_eq!(second.service_future_created_at_ms, Some(1_700_000_000_020));
    assert_eq!(second.service_scope_entered_at_ms, Some(1_700_000_000_021));

    let first = take_completion_request_context(&uri, position)
        .expect("first request should remain available by position");
    assert_eq!(first.request_id, first_request_id);
    assert_eq!(first.request_received_at_ms, Some(1_700_000_000_010));
    assert_eq!(first.service_future_created_at_ms, Some(1_700_000_000_011));
    assert_eq!(first.service_scope_entered_at_ms, Some(1_700_000_000_012));
}

#[test]
fn pending_completion_request_is_removed_when_cancelled_before_take() {
    let uri = Url::parse("file:///request_context_cancelled.bsl").expect("url");
    let request_id = "req-cancel";
    let request = Request::build("textDocument/completion")
        .id(request_id)
        .params(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 2 },
        }))
        .finish();
    record_pending_completion_request_id(&request, request_id, None);
    remove_pending_completion_request_id(request_id);

    let taken = take_completion_request_id(
        &Url::parse("file:///request_context_cancelled.bsl").expect("url"),
        Position::new(1, 2),
    );
    assert_eq!(taken, None);
}

#[test]
fn cancelled_request_id_extracted_for_numeric_and_string_ids() {
    let numeric = Request::build("$/cancelRequest")
        .params(json!({ "id": 7 }))
        .finish();
    assert_eq!(
        cancelled_request_id_from_request(&numeric),
        Some("7".to_string())
    );

    let string = Request::build("$/cancelRequest")
        .params(json!({ "id": "r42" }))
        .finish();
    assert_eq!(
        cancelled_request_id_from_request(&string),
        Some("r42".to_string())
    );

    let non_cancel = Request::build("textDocument/completion").id(1_i64).finish();
    assert_eq!(cancelled_request_id_from_request(&non_cancel), None);
}
