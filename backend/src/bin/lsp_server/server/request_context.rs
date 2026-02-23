use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tower::Service;
use tower_lsp::jsonrpc::{Id, Request};
use tower_lsp::lsp_types::{CancelParams, CompletionParams, NumberOrString, Position, Url};

tokio::task_local! {
    static LSP_REQUEST_ID: Option<String>;
}

type CancelRequestHook = Arc<dyn Fn(String) + Send + Sync + 'static>;

fn cancel_request_hook_cell() -> &'static Mutex<Option<CancelRequestHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CancelRequestHook>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CompletionRequestKey {
    uri: String,
    line: u32,
    character: u32,
}

#[derive(Debug, Default)]
struct PendingCompletionRequestIds {
    by_key: HashMap<CompletionRequestKey, VecDeque<String>>,
    by_request_id: HashMap<String, CompletionRequestKey>,
}

fn pending_completion_request_ids_cell() -> &'static Mutex<PendingCompletionRequestIds> {
    static CELL: std::sync::OnceLock<Mutex<PendingCompletionRequestIds>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(PendingCompletionRequestIds::default()))
}

fn completion_request_key(params: &CompletionParams) -> CompletionRequestKey {
    let text_document_position = &params.text_document_position;
    CompletionRequestKey {
        uri: text_document_position.text_document.uri.to_string(),
        line: text_document_position.position.line,
        character: text_document_position.position.character,
    }
}

fn record_pending_completion_request_id(request: &Request, request_id: &str) {
    if request.method() != "textDocument/completion" {
        return;
    }
    let Some(params) = request.params().cloned() else {
        return;
    };
    let Ok(completion_params) = serde_json::from_value::<CompletionParams>(params) else {
        return;
    };
    let key = completion_request_key(&completion_params);
    let request_id = request_id.to_string();
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old_key) = pending
        .by_request_id
        .insert(request_id.clone(), key.clone())
    {
        if let Some(old_queue) = pending.by_key.get_mut(&old_key) {
            old_queue.retain(|queued| queued != &request_id);
            if old_queue.is_empty() {
                pending.by_key.remove(&old_key);
            }
        }
    }
    pending.by_key.entry(key).or_default().push_back(request_id);
}

fn remove_pending_completion_request_id(request_id: &str) {
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(key) = pending.by_request_id.remove(request_id) else {
        return;
    };
    if let Some(queue) = pending.by_key.get_mut(&key) {
        queue.retain(|queued| queued != request_id);
        if queue.is_empty() {
            pending.by_key.remove(&key);
        }
    }
}

pub(crate) fn take_completion_request_id(uri: &Url, position: Position) -> Option<String> {
    let key = CompletionRequestKey {
        uri: uri.to_string(),
        line: position.line,
        character: position.character,
    };
    let mut pending = pending_completion_request_ids_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loop {
        let request_id = {
            let queue = pending.by_key.get_mut(&key)?;
            queue.pop_front()
        };
        let Some(request_id) = request_id else {
            pending.by_key.remove(&key);
            return None;
        };
        if pending.by_request_id.remove(&request_id).is_some() {
            let empty = pending
                .by_key
                .get(&key)
                .is_some_and(|queue| queue.is_empty());
            if empty {
                pending.by_key.remove(&key);
            }
            return Some(request_id);
        }
    }
}

pub(crate) fn current_request_id() -> Option<String> {
    LSP_REQUEST_ID.try_with(Clone::clone).ok().flatten()
}

pub(crate) fn set_cancel_request_hook(hook: Option<CancelRequestHook>) {
    let mut slot = cancel_request_hook_cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = hook;
}

async fn with_request_id<F, T>(request_id: Option<String>, future: F) -> T
where
    F: Future<Output = T>,
{
    LSP_REQUEST_ID.scope(request_id, future).await
}

fn request_id_from_jsonrpc_id(id: &Id) -> Option<String> {
    match id {
        Id::Number(value) => Some(value.to_string()),
        Id::String(value) => Some(value.clone()),
        Id::Null => None,
    }
}

fn request_id_from_request(request: &Request) -> Option<String> {
    request.id().and_then(request_id_from_jsonrpc_id)
}

fn cancelled_request_id_from_request(request: &Request) -> Option<String> {
    if request.method() != "$/cancelRequest" {
        return None;
    }
    let params = request.params()?.clone();
    let cancel_params: CancelParams = serde_json::from_value(params).ok()?;
    match cancel_params.id {
        NumberOrString::Number(value) => Some(value.to_string()),
        NumberOrString::String(value) => Some(value),
    }
}

fn notify_cancel_request_hook(request_id: String) {
    let hook = {
        let slot = cancel_request_hook_cell()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.clone()
    };
    if let Some(hook) = hook {
        hook(request_id);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RequestContextService<S> {
    inner: S,
}

impl<S> RequestContextService<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<Request> for RequestContextService<S>
where
    S: Service<Request> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if let Some(request_id) = cancelled_request_id_from_request(&request) {
            remove_pending_completion_request_id(&request_id);
            notify_cancel_request_hook(request_id);
        }
        let request_id = request_id_from_request(&request);
        if let Some(request_id) = request_id.as_deref() {
            record_pending_completion_request_id(&request, request_id);
        }
        let future = self.inner.call(request);
        Box::pin(async move { with_request_id(request_id, future).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn current_request_id_is_none_outside_scope() {
        assert_eq!(current_request_id(), None);
    }

    #[tokio::test]
    async fn with_request_id_exposes_context_inside_scope() {
        let scoped = with_request_id(Some("42".to_string()), async { current_request_id() }).await;
        assert_eq!(scoped, Some("42".to_string()));
    }

    #[tokio::test]
    async fn request_context_service_sets_jsonrpc_numeric_id() {
        #[derive(Clone, Debug, Default)]
        struct CaptureService;

        impl Service<Request> for CaptureService {
            type Response = Option<String>;
            type Error = ();
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _request: Request) -> Self::Future {
                Box::pin(async move { Ok(current_request_id()) })
            }
        }

        let mut service = RequestContextService::new(CaptureService);
        let request = Request::build("workspace/symbol").id(9_i64).finish();
        let captured = service.call(request).await.expect("service call");
        assert_eq!(captured, Some("9".to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn request_context_service_does_not_propagate_request_id_to_spawned_handler() {
        #[derive(Clone, Debug, Default)]
        struct SpawnedCaptureService;

        impl Service<Request> for SpawnedCaptureService {
            type Response = Option<String>;
            type Error = ();
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }

            fn call(&mut self, _request: Request) -> Self::Future {
                Box::pin(async move {
                    let captured = tokio::spawn(async move { current_request_id() })
                        .await
                        .expect("spawned capture join");
                    Ok(captured)
                })
            }
        }

        let mut service = RequestContextService::new(SpawnedCaptureService);
        let request = Request::build("workspace/symbol").id(77_i64).finish();
        let captured = service.call(request).await.expect("service call");
        assert_eq!(captured, None);
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
}
