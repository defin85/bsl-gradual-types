use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tower::Service;
use tower_lsp::jsonrpc::{Id, Request};

tokio::task_local! {
    static LSP_REQUEST_ID: Option<String>;
}

pub(crate) fn current_request_id() -> Option<String> {
    LSP_REQUEST_ID.try_with(Clone::clone).ok().flatten()
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
        let request_id = request_id_from_request(&request);
        let future = self.inner.call(request);
        Box::pin(async move { with_request_id(request_id, future).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
