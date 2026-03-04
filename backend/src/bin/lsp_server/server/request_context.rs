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
#[path = "request_context/tests.rs"]
mod tests;
