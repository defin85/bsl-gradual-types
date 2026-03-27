use tower::Service;
use tower_lsp::jsonrpc::{Request, Response};
use tower_lsp::{Loopback, Server};

pub(crate) async fn serve_with_completion_handoff<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    service: S,
    concurrency_level: usize,
) where
    I: tokio::io::AsyncRead + Unpin,
    O: tokio::io::AsyncWrite,
    L: Loopback,
    <L::ResponseSink as futures::Sink<Response>>::Error: std::error::Error + Send + Sync + 'static,
    S: Service<Request, Response = Option<Response>> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send,
{
    // Task 1 only moves the transport boundary behind a project-owned seam.
    // The completion-specific deferred handoff is added on top of this entry point.
    Server::new(stdin, stdout, socket)
        .concurrency_level(concurrency_level)
        .serve(service)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tower_lsp::jsonrpc::Response as JsonRpcResponse;

    #[derive(Debug)]
    struct EchoService;

    impl Service<Request> for EchoService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let id = request.id().expect("request id").clone();
            Box::pin(async move {
                Ok(Some(JsonRpcResponse::from_ok(
                    id,
                    json!({ "capabilities": {} }),
                )))
            })
        }
    }

    struct NullLoopback;

    impl Loopback for NullLoopback {
        type RequestStream = futures::stream::Pending<Request>;
        type ResponseSink = futures::sink::Drain<Response>;

        fn split(self) -> (Self::RequestStream, Self::ResponseSink) {
            (futures::stream::pending(), futures::sink::drain())
        }
    }

    async fn read_framed_message(
        reader: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    ) -> serde_json::Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes = reader
                .read_line(&mut line)
                .await
                .expect("read response header line");
            assert!(bytes > 0, "unexpected EOF while reading response header");
            if line == "\r\n" {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    raw_len
                        .trim()
                        .parse::<usize>()
                        .expect("parse response content length"),
                );
            }
        }
        let body_len = content_length.expect("response content length");
        let mut body = vec![0; body_len];
        reader
            .read_exact(&mut body)
            .await
            .expect("read response body");
        serde_json::from_slice(&body).expect("parse response json")
    }

    #[tokio::test]
    async fn transport_adapter_forwards_jsonrpc_response_over_stdio() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);

        let server_task = tokio::spawn(async move {
            serve_with_completion_handoff(server_read, server_write, NullLoopback, EchoService, 2)
                .await;
        });

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let body = serde_json::to_vec(&request).expect("serialize request");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        client_write
            .write_all(header.as_bytes())
            .await
            .expect("write request header");
        client_write
            .write_all(&body)
            .await
            .expect("write request body");
        client_write.flush().await.expect("flush request");

        let mut reader = BufReader::new(client_read);
        let response = read_framed_message(&mut reader).await;
        assert_eq!(response.get("id").and_then(|value| value.as_i64()), Some(1));
        assert_eq!(
            response
                .get("result")
                .and_then(|value| value.get("capabilities"))
                .and_then(|value| value.as_object())
                .map(|map| map.is_empty()),
            Some(true)
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }
}
