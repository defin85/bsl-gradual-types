use std::fmt::{self, Display, Formatter};
use std::io::Error as IoError;
use std::num::ParseIntError;
use std::str::Utf8Error;

use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::{future, pin_mut, stream, FutureExt, Sink, SinkExt, StreamExt, TryFutureExt};
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tower::Service;
use tower_lsp::jsonrpc::{Error, Id, Request, Response};
use tower_lsp::Loopback;
use tracing::error;

const MESSAGE_QUEUE_SIZE: usize = 100;
const COMPLETION_METHOD: &str = "textDocument/completion";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum TransportMessage {
    Response(Response),
    Request(Request),
}

#[derive(Debug)]
enum TransportCodecError {
    Json(serde_json::Error),
    Io(IoError),
    InvalidContentLength(ParseIntError),
    InvalidContentType,
    MissingContentLength,
    UnexpectedEof,
    Utf8(Utf8Error),
}

impl Display for TransportCodecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(err) => write!(f, "unable to parse JSON body: {err}"),
            Self::Io(err) => write!(f, "failed to process transport frame: {err}"),
            Self::InvalidContentLength(err) => {
                write!(f, "unable to parse content length: {err}")
            }
            Self::InvalidContentType => write!(f, "unable to parse content type"),
            Self::MissingContentLength => write!(f, "missing required `Content-Length` header"),
            Self::UnexpectedEof => write!(f, "unexpected EOF while reading transport message"),
            Self::Utf8(err) => write!(f, "request contains invalid UTF8: {err}"),
        }
    }
}

impl std::error::Error for TransportCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::InvalidContentLength(err) => Some(err),
            Self::Utf8(err) => Some(err),
            Self::InvalidContentType | Self::MissingContentLength | Self::UnexpectedEof => None,
        }
    }
}

impl From<IoError> for TransportCodecError {
    fn from(error: IoError) -> Self {
        Self::Io(error)
    }
}

impl From<ParseIntError> for TransportCodecError {
    fn from(error: ParseIntError) -> Self {
        Self::InvalidContentLength(error)
    }
}

impl From<Utf8Error> for TransportCodecError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

struct CompletionHandoffTask {
    request_id: Option<String>,
    future: BoxFuture<'static, Option<Response>>,
}

impl CompletionHandoffTask {
    fn new(request_id: Option<String>, future: BoxFuture<'static, Option<Response>>) -> Self {
        Self { request_id, future }
    }

    async fn forward_response(self, mut responses_tx: mpsc::Sender<TransportMessage>) {
        let request_id = self.request_id;
        if let Some(response) = self.future.await {
            if responses_tx
                .send(TransportMessage::Response(response))
                .await
                .is_err()
            {
                match request_id.as_deref() {
                    Some(request_id) => error!(
                        "failed to forward deferred completion response for request {request_id}: transport closed"
                    ),
                    None => error!(
                        "failed to forward deferred completion response without request id: transport closed"
                    ),
                }
            }
        }
    }
}

pub(crate) async fn serve_with_completion_handoff<I, O, L, S>(
    stdin: I,
    stdout: O,
    socket: L,
    mut service: S,
    concurrency_level: usize,
) where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    L: Loopback,
    <L::ResponseSink as Sink<Response>>::Error: std::error::Error + Send + Sync + 'static,
    S: Service<Request, Response = Option<Response>> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
{
    let (client_requests, mut client_responses) = socket.split();
    let (client_requests, client_abort) = stream::abortable(client_requests);
    let (mut responses_tx, responses_rx) = mpsc::channel(0);
    let (mut server_tasks_tx, server_tasks_rx) =
        mpsc::channel::<BoxFuture<'static, Option<Response>>>(MESSAGE_QUEUE_SIZE);
    let (mut completion_tasks_tx, completion_tasks_rx) =
        mpsc::channel::<CompletionHandoffTask>(MESSAGE_QUEUE_SIZE);
    let transport_shutdown = std::sync::Arc::new(Notify::new());

    let responses_tx_for_server_tasks = responses_tx.clone();
    let process_server_tasks = async move {
        let mut responses_tx = responses_tx_for_server_tasks;
        let mut server_tasks = server_tasks_rx.buffer_unordered(concurrency_level);

        while let Some(response) = server_tasks.next().await {
            let Some(response) = response else {
                continue;
            };
            if responses_tx
                .send(TransportMessage::Response(response))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    let responses_tx_for_completion = responses_tx.clone();
    let process_completion_tasks = async move {
        let mut completion_tasks = JoinSet::new();
        let mut completion_tasks_rx = completion_tasks_rx.fuse();
        let mut receiver_closed = false;

        loop {
            tokio::select! {
                maybe_task = completion_tasks_rx.next(), if !receiver_closed => {
                    match maybe_task {
                        Some(task) => {
                            let responses_tx = responses_tx_for_completion.clone();
                            completion_tasks.spawn(async move {
                                task.forward_response(responses_tx).await;
                            });
                        }
                        None => {
                            receiver_closed = true;
                            completion_tasks.abort_all();
                            if completion_tasks.is_empty() {
                                break;
                            }
                        }
                    }
                }
                join_result = completion_tasks.join_next(), if !completion_tasks.is_empty() => {
                    if let Some(Err(err)) = join_result {
                        if !err.is_cancelled() {
                            error!("completion handoff task failed: {err}");
                        }
                    }
                }
                else => {
                    if receiver_closed {
                        break;
                    }
                }
            }
        }
    };

    let transport_shutdown_for_output = transport_shutdown.clone();
    let print_output = async move {
        let mut stdout = BufWriter::new(stdout);
        let outbound = stream::select(responses_rx, client_requests.map(TransportMessage::Request));
        pin_mut!(outbound);

        loop {
            tokio::select! {
                _ = transport_shutdown_for_output.notified() => break,
                maybe_message = outbound.next() => {
                    let Some(message) = maybe_message else {
                        break;
                    };
                    if let Err(err) = write_transport_message(&mut stdout, &message).await {
                        error!("failed to encode message: {err}");
                        break;
                    }
                }
            }
        }
    };

    let read_input = async move {
        let mut stdin = BufReader::new(stdin);

        loop {
            match read_transport_message(&mut stdin).await {
                Ok(Some(TransportMessage::Request(request))) => {
                    if let Err(err) = future::poll_fn(|cx| service.poll_ready(cx)).await {
                        error!("{}", display_sources(err.into().as_ref()));
                        break;
                    }

                    let request_id = request.id().map(ToString::to_string);
                    let is_completion = is_completion_request(&request);
                    let future = service
                        .call(request)
                        .unwrap_or_else(|err| {
                            error!("{}", display_sources(err.into().as_ref()));
                            None
                        })
                        .boxed();

                    if is_completion {
                        let task = CompletionHandoffTask::new(request_id, future);
                        if completion_tasks_tx.send(task).await.is_err() {
                            error!("completion handoff queue closed unexpectedly");
                            break;
                        }
                    } else if server_tasks_tx.send(future).await.is_err() {
                        error!("server task queue closed unexpectedly");
                        break;
                    }
                }
                Ok(Some(TransportMessage::Response(response))) => {
                    if let Err(err) = client_responses.send(response).await {
                        error!("{}", display_sources(&err));
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    error!("failed to decode message: {err}");
                    let response = Response::from_error(Id::Null, to_jsonrpc_error(&err));
                    if responses_tx
                        .send(TransportMessage::Response(response))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }

        transport_shutdown.notify_waiters();
        server_tasks_tx.disconnect();
        completion_tasks_tx.disconnect();
        responses_tx.disconnect();
        client_abort.abort();
    };

    futures::join!(
        print_output,
        read_input,
        process_server_tasks,
        process_completion_tasks
    );
}

fn is_completion_request(request: &Request) -> bool {
    request.method() == COMPLETION_METHOD && request.id().is_some()
}

async fn read_transport_message<I>(
    reader: &mut BufReader<I>,
) -> Result<Option<TransportMessage>, TransportCodecError>
where
    I: AsyncRead + Unpin,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).await?;

        if bytes == 0 {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(TransportCodecError::UnexpectedEof)
            };
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(raw_len.trim().parse::<usize>()?);
            continue;
        }

        if let Some(raw_content_type) = trimmed.strip_prefix("Content-Type:") {
            validate_content_type(raw_content_type.trim())?;
        }
    }

    let body_len = content_length.ok_or(TransportCodecError::MissingContentLength)?;
    let mut body = vec![0; body_len];
    reader.read_exact(&mut body).await?;
    let message = serde_json::from_slice(&body).map_err(TransportCodecError::Json)?;
    Ok(Some(message))
}

async fn write_transport_message<O>(
    writer: &mut BufWriter<O>,
    message: &TransportMessage,
) -> Result<(), TransportCodecError>
where
    O: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(message).map_err(TransportCodecError::Json)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

fn validate_content_type(content_type: &str) -> Result<(), TransportCodecError> {
    let charset = content_type
        .split(';')
        .skip(1)
        .map(str::trim)
        .find_map(|param| param.strip_prefix("charset="));

    match charset {
        Some("utf-8") | Some("utf8") | None => Ok(()),
        Some(_) => Err(TransportCodecError::InvalidContentType),
    }
}

fn display_sources(error: &dyn std::error::Error) -> String {
    if let Some(source) = error.source() {
        format!("{error}: {}", display_sources(source))
    } else {
        error.to_string()
    }
}

fn to_jsonrpc_error(err: &TransportCodecError) -> Error {
    match err {
        TransportCodecError::Json(err) if err.is_data() => Error::invalid_request(),
        TransportCodecError::Json(_)
        | TransportCodecError::Io(_)
        | TransportCodecError::InvalidContentLength(_)
        | TransportCodecError::InvalidContentType
        | TransportCodecError::MissingContentLength
        | TransportCodecError::UnexpectedEof
        | TransportCodecError::Utf8(_) => Error::parse_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::Notify;
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

    #[derive(Debug, Clone)]
    struct BlockingCompletionService {
        completion_release: std::sync::Arc<Notify>,
    }

    impl Service<Request> for BlockingCompletionService {
        type Response = Option<Response>;
        type Error = std::convert::Infallible;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let request_id = request.id().expect("request id").clone();
            let method = request.method().to_string();
            let completion_release = self.completion_release.clone();
            Box::pin(async move {
                if method == "textDocument/completion" {
                    completion_release.notified().await;
                    return Ok(Some(JsonRpcResponse::from_ok(
                        request_id,
                        json!({ "items": [], "isIncomplete": false }),
                    )));
                }

                Ok(Some(JsonRpcResponse::from_ok(
                    request_id,
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

    #[tokio::test]
    async fn transport_adapter_releases_ingress_slot_before_blocking_completion_wait() {
        let (client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (client_read, mut client_write) = tokio::io::split(client_stream);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let completion_release = std::sync::Arc::new(Notify::new());

        let server_task = tokio::spawn({
            let completion_release = completion_release.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    BlockingCompletionService { completion_release },
                    1,
                )
                .await;
            }
        });

        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": "file:///test.bsl" },
                    "position": { "line": 0, "character": 0 }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": {}
            }),
        ] {
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
        }
        client_write.flush().await.expect("flush requests");

        let mut reader = BufReader::new(client_read);
        let first_response = tokio::time::timeout(
            std::time::Duration::from_millis(150),
            read_framed_message(&mut reader),
        )
        .await
        .expect("initialize response must not wait for completion release");
        assert_eq!(
            first_response.get("id").and_then(|value| value.as_i64()),
            Some(2)
        );

        completion_release.notify_waiters();
        let second_response = read_framed_message(&mut reader).await;
        assert_eq!(
            second_response.get("id").and_then(|value| value.as_i64()),
            Some(1)
        );
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                read_framed_message(&mut reader),
            )
            .await
            .is_err(),
            "completion handoff must not emit duplicate terminal responses"
        );

        drop(client_write);
        server_task.abort();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn transport_adapter_aborts_blocked_completion_handoff_on_transport_shutdown() {
        let (mut client_stream, server_stream) = tokio::io::duplex(8 * 1024);
        let (server_read, server_write) = tokio::io::split(server_stream);
        let completion_release = std::sync::Arc::new(Notify::new());

        let server_task = tokio::spawn({
            let completion_release = completion_release.clone();
            async move {
                serve_with_completion_handoff(
                    server_read,
                    server_write,
                    NullLoopback,
                    BlockingCompletionService { completion_release },
                    1,
                )
                .await;
            }
        });

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///test.bsl" },
                "position": { "line": 0, "character": 0 }
            }
        });
        let body = serde_json::to_vec(&request).expect("serialize request");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        client_stream
            .write_all(header.as_bytes())
            .await
            .expect("write request header");
        client_stream
            .write_all(&body)
            .await
            .expect("write request body");
        client_stream.flush().await.expect("flush request");

        drop(client_stream);
        tokio::time::timeout(std::time::Duration::from_secs(1), server_task)
            .await
            .expect("transport shutdown must abort blocked completion handoff")
            .expect("server task must exit cleanly");
    }
}
