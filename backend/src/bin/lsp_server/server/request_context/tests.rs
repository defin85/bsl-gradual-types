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
async fn with_request_context_exposes_context_inside_scope() {
    let scoped = with_request_context(
        Some("42".to_string()),
        Some(1_700_000_000_123),
        async { (current_request_id(), current_request_received_at_ms()) },
    )
    .await;
    assert_eq!(scoped.0, Some("42".to_string()));
    assert_eq!(scoped.1, Some(1_700_000_000_123));
}

#[tokio::test]
async fn request_context_service_sets_jsonrpc_numeric_id() {
    #[derive(Clone, Debug, Default)]
    struct CaptureService;

    impl Service<Request> for CaptureService {
        type Response = (Option<String>, Option<u64>);
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            Box::pin(async move { Ok((current_request_id(), current_request_received_at_ms())) })
        }
    }

    let mut service = RequestContextService::new(CaptureService);
    let request = Request::build("workspace/symbol").id(9_i64).finish();
    let captured = service.call(request).await.expect("service call");
    assert_eq!(captured.0, Some("9".to_string()));
    assert!(captured.1.is_some(), "request receive timestamp must be scoped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_context_service_does_not_propagate_request_id_to_spawned_handler() {
    #[derive(Clone, Debug, Default)]
    struct SpawnedCaptureService;

    impl Service<Request> for SpawnedCaptureService {
        type Response = (Option<String>, Option<u64>);
        type Error = ();
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: Request) -> Self::Future {
            Box::pin(async move {
                let captured = tokio::spawn(async move {
                    (current_request_id(), current_request_received_at_ms())
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
    record_pending_completion_request_id(&request, request_id);

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
    record_pending_completion_request_id(&request, request_id);
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
