use std::{collections::VecDeque, process::Stdio, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream},
    process::{Child, ChildStdout, Command},
    sync::Mutex,
    time::timeout,
};

/// Default timeout while waiting for the adapter to report a listening port.
const ADAPTER_PORT_TIMEOUT: Duration = Duration::from_secs(15);

/// DAP event payloads passed around in the session.
pub type DapEvent = Value;

/// Defines where and how we connect to the debug adapter.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AdapterEndpoint {
    /// Spawn the adapter (CodeLLDB by default) and connect to the announced TCP port.
    Spawn {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default = "default_adapter_host")]
        host: String,
    },
    /// Connect directly to an already running adapter.
    DirectTcp {
        #[serde(default = "default_adapter_host")]
        host: String,
        port: u16,
    },
}

fn default_adapter_host() -> String {
    "127.0.0.1".to_string()
}

impl Default for AdapterEndpoint {
    fn default() -> Self {
        AdapterEndpoint::Spawn {
            command: "codelldb".to_string(),
            args: vec!["--port".to_string(), "0".to_string()],
            host: default_adapter_host(),
        }
    }
}

/// Wrapper used by sessions to keep adapter configuration handy.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AdapterSettings {
    #[serde(flatten)]
    pub endpoint: AdapterEndpoint,
}

/// Minimal DAP client capable of sending requests and buffering asynchronous events.
pub struct DapClient {
    writer: OwnedWriteHalf,
    reader: BufReader<OwnedReadHalf>,
    seq_counter: u32,
    /// Optional spawned adapter process (CodeLLDB). We keep it to shut down gracefully.
    adapter_child: Option<Arc<Mutex<Child>>>,
}

impl DapClient {
    /// Establishes a connection either by spawning the adapter or connecting to an existing socket.
    pub async fn connect(settings: AdapterSettings) -> Result<Self> {
        match settings.endpoint {
            AdapterEndpoint::Spawn { command, args, host } => {
                let (stream, child) = spawn_adapter(&command, &args, &host).await?;
                let (reader, writer) = split_stream(stream);
                Ok(Self {
                    writer,
                    reader,
                    seq_counter: 1,
                    adapter_child: Some(Arc::new(Mutex::new(child))),
                })
            }
            AdapterEndpoint::DirectTcp { host, port } => {
                let addr = (host.as_str(), port);
                let stream = TcpStream::connect(addr)
                    .await
                    .with_context(|| format!("Не удалось подключиться к DAP по адресу {}:{}", host, port))?;
                let (reader, writer) = split_stream(stream);
                Ok(Self {
                    writer,
                    reader,
                    seq_counter: 1,
                    adapter_child: None,
                })
            }
        }
    }

    /// Sends a DAP request and waits for the matching response.
    /// Any asynchronous events received during the wait are appended into `events`.
    pub async fn send_request(
        &mut self,
        command: &str,
        arguments: Value,
        events: &mut VecDeque<DapEvent>,
    ) -> Result<Value> {
        let seq = self.next_seq();
        let payload = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments
        });
        self.write_payload(&payload).await?;

        loop {
            match self.read_message().await? {
                DapPacket::Response(resp) => {
                    let request_seq = resp
                        .get("request_seq")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| anyhow!("Ответ DAP без request_seq: {resp}"))?;
                    if request_seq as u32 != seq {
                        // Неожиданно, но продолжаем искать нужный ответ.
                        continue;
                    }

                    if resp.get("success").and_then(Value::as_bool) == Some(false) {
                        let message = resp
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("DAP adapter reported failure");
                        bail!("Команда DAP `{command}` завершилась ошибкой: {message}");
                    }

                    return Ok(resp.get("body").cloned().unwrap_or(Value::Null));
                }
                DapPacket::Event(event) => events.push_back(event),
                DapPacket::Request(request) => {
                    tracing::warn!(
                        target: "mcp-debug-server::dap",
                        "Получен неожиданный DAP request: {request}"
                    );
                    // Формируем отрицательный ответ, чтобы CodeLLDB не завис.
                    if let Some(seq) = request.get("seq").and_then(Value::as_u64) {
                        let response = json!({
                            "type": "response",
                            "request_seq": seq,
                            "success": false,
                            "command": request.get("command").cloned().unwrap_or(Value::Null),
                            "message": "MCP Debug Server не поддерживает данный запрос"
                        });
                        self.write_payload(&response).await?;
                    }
                }
            }
        }
    }

    /// Blocks until an event matching the predicate arrives.
    pub async fn wait_for_event<F>(
        &mut self,
        events: &mut VecDeque<DapEvent>,
        predicate: F,
    ) -> Result<DapEvent>
    where
        F: Fn(&DapEvent) -> bool,
    {
        if let Some(idx) = events.iter().position(|event| predicate(event)) {
            return Ok(events.remove(idx).unwrap());
        }

        loop {
            match self.read_message().await? {
                DapPacket::Event(event) => {
                    if predicate(&event) {
                        return Ok(event);
                    }
                    events.push_back(event);
                }
                DapPacket::Response(resp) => {
                    tracing::debug!(
                        target: "mcp-debug-server::dap",
                        "Игнорирую неожиданно полученный response: {}",
                        resp
                    );
                }
                DapPacket::Request(req) => {
                    tracing::warn!(
                        target: "mcp-debug-server::dap",
                        "Получен запрос в wait_for_event: {}",
                        req
                    );
                    if let Some(seq) = req.get("seq").and_then(Value::as_u64) {
                        let response = json!({
                            "type": "response",
                            "request_seq": seq,
                            "success": false,
                            "command": req.get("command").cloned().unwrap_or(Value::Null),
                            "message": "wait_for_event: клиент не поддерживает этот запрос"
                        });
                        self.write_payload(&response).await?;
                    }
                }
            }
        }
    }

    fn next_seq(&mut self) -> u32 {
        let seq = self.seq_counter;
        self.seq_counter += 1;
        seq
    }

    async fn write_payload(&mut self, payload: &Value) -> Result<()> {
        let body = serde_json::to_string(payload)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(body.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<DapPacket> {
        let mut content_length: Option<usize> = None;
        loop {
            let mut header = String::new();
            if self.reader.read_line(&mut header).await? == 0 {
                bail!("DAP-соединение закрыто");
            }
            let trimmed = header.trim();
            if trimmed.is_empty() {
                break;
            }

            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(rest.trim().parse()?);
            }
        }

        let length = content_length.ok_or_else(|| anyhow!("Content-Length не найден"))?;
        let mut buffer = vec![0u8; length];
        self.reader.read_exact(&mut buffer).await?;
        let value: Value = serde_json::from_slice(&buffer)?;
        Ok(DapPacket::from_value(value)?)
    }
}

impl Drop for DapClient {
    fn drop(&mut self) {
        if let Some(child) = &self.adapter_child {
            if let Ok(mut guard) = child.try_lock() {
                let _ = guard.start_kill();
            }
        }
    }
}

enum DapPacket {
    Event(Value),
    Response(Value),
    Request(Value),
}

impl DapPacket {
    fn from_value(value: Value) -> Result<Self> {
        match value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("DAP сообщение без поля `type`: {value}"))?
        {
            "event" => Ok(DapPacket::Event(value)),
            "response" => Ok(DapPacket::Response(value)),
            "request" => Ok(DapPacket::Request(value)),
            other => bail!("Неизвестный тип DAP сообщения: {other}"),
        }
    }
}

async fn spawn_adapter(
    command: &str,
    args: &[String],
    host: &str,
) -> Result<(TcpStream, Child)> {
    let mut cmd = Command::new(command);
    cmd.args(resolve_args(args));
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Не удалось запустить адаптер `{command}`"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Не удалось получить stdout адаптера"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("Не удалось получить stderr адаптера"))?;

    let mut reader = BufReader::new(stdout);
    let port = timeout(ADAPTER_PORT_TIMEOUT, read_port(&mut reader))
        .await
        .context("Таймаут ожидания порта от CodeLLDB")??;

    // Продолжаем читать логи адаптера в фоне, чтобы не блокировать stdout/stderr.
    spawn_log_task(reader, "stdout");
    spawn_log_task(BufReader::new(stderr), "stderr");

    tracing::info!("🔌 CodeLLDB слушает на {host}:{port}");
    let stream = TcpStream::connect((host, port))
        .await
        .with_context(|| format!("Не удалось подключиться к CodeLLDB по адресу {host}:{port}"))?;
    Ok((stream, child))
}

fn split_stream(stream: TcpStream) -> (BufReader<OwnedReadHalf>, OwnedWriteHalf) {
    let (reader, writer) = stream.into_split();
    (BufReader::new(reader), writer)
}

async fn read_port(reader: &mut BufReader<ChildStdout>) -> Result<u16> {
    let regex = Regex::new(r"(?:port|tcp://[0-9.]+:)\s*(\d+)").unwrap();
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            bail!("CodeLLDB завершился, не сообщив порт");
        }
        if let Some(caps) = regex.captures(&line) {
            let port: u16 = caps[1].parse()?;
            return Ok(port);
        }
        tracing::debug!(
            target: "mcp-debug-server::dap",
            "Вывод CodeLLDB: {}",
            line.trim()
        );
    }
}

fn resolve_args(args: &[String]) -> Vec<String> {
    if args.iter().any(|arg| arg == "--port" || arg.starts_with("--port=")) {
        return args.to_vec();
    }

    let mut result = args.to_vec();
    result.push("--port".to_string());
    result.push("0".to_string());
    result
}

fn spawn_log_task<R>(mut reader: BufReader<R>, label: &'static str)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    tracing::debug!(
                        target: "mcp-debug-server::dap",
                        "[adapter-{label}] {}",
                        line.trim()
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        target: "mcp-debug-server::dap",
                        "Ошибка чтения логов адаптера ({label}): {err:?}"
                    );
                    break;
                }
            }
        }
    });
}
