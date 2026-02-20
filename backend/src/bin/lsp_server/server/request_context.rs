use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tower::Service;
use tower_lsp::jsonrpc::{Id, Request};
use tower_lsp::lsp_types::{CancelParams, NumberOrString};

tokio::task_local! {
    static LSP_REQUEST_ID: Option<String>;
}

type CancelRequestHook = Arc<dyn Fn(String) + Send + Sync + 'static>;

fn cancel_request_hook_cell() -> &'static Mutex<Option<CancelRequestHook>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CancelRequestHook>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
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
            notify_cancel_request_hook(request_id);
        }
        let request_id = request_id_from_request(&request);
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
        let request = Request::build("textDocument/completion").id(9_i64).finish();
        let captured = service.call(request).await.expect("service call");
        assert_eq!(captured, Some("9".to_string()));
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
