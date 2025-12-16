#![allow(dead_code)]

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Mock DAP Server для интеграционного тестирования
///
/// Имитирует поведение реального DAP adapter (например, CodeLLDB)
/// без необходимости запускать настоящий debugger
pub struct MockDapServer {
    listener: TcpListener,
    state: Arc<RwLock<ServerState>>,
}

#[derive(Debug, Default)]
struct ServerState {
    /// Счетчик sequence для responses
    response_seq: u32,

    /// Флаг инициализации
    initialized: bool,

    /// Запущена ли программа
    launched: bool,

    /// Установленные breakpoints: file -> [lines]
    breakpoints: std::collections::HashMap<String, Vec<u32>>,

    /// Текущий thread ID
    thread_id: u32,
}

impl MockDapServer {
    /// Создать новый mock DAP server
    pub async fn new() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        Ok(Self {
            listener,
            state: Arc::new(RwLock::new(ServerState::default())),
        })
    }

    /// Получить порт, на котором слушает сервер
    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }

    /// Получить адрес сервера в формате "127.0.0.1:PORT"
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port())
    }

    /// Запустить mock server (блокирующий вызов)
    pub async fn run(self) {
        tracing::info!("Mock DAP server started on port {}", self.port());

        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    tracing::debug!("Accepted connection from {}", addr);
                    let state = Arc::clone(&self.state);

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(socket, state).await {
                            tracing::error!("Client handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                    break;
                }
            }
        }
    }

    /// Обработка клиентского соединения
    async fn handle_client(
        mut socket: tokio::net::TcpStream,
        state: Arc<RwLock<ServerState>>,
    ) -> std::io::Result<()> {
        let mut buffer = vec![0u8; 8192];

        loop {
            // Читаем заголовок Content-Length
            let mut header_buf = Vec::new();
            loop {
                let n = socket.read(&mut buffer[..1]).await?;
                if n == 0 {
                    return Ok(());
                }
                header_buf.push(buffer[0]);

                // Проверяем конец заголовков (\r\n\r\n)
                if header_buf.len() >= 4 && &header_buf[header_buf.len() - 4..] == b"\r\n\r\n" {
                    break;
                }
            }

            // Парсим Content-Length
            let header_str = String::from_utf8_lossy(&header_buf);
            let content_length = header_str
                .lines()
                .find(|line| line.starts_with("Content-Length:"))
                .and_then(|line| line.split(':').nth(1))
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0);

            if content_length == 0 {
                tracing::error!("Invalid Content-Length");
                return Ok(());
            }

            // Читаем тело сообщения
            let mut body = vec![0u8; content_length];
            socket.read_exact(&mut body).await?;

            // Парсим DAP request
            let request: Value = match serde_json::from_slice(&body) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Failed to parse DAP request: {}", e);
                    continue;
                }
            };

            tracing::debug!("Received DAP request: {}", request);

            // Обработка request и формирование response
            let response = Self::handle_request(request, &state).await;

            // Отправка response
            Self::send_message(&mut socket, &response).await?;
        }
    }

    /// Обработка DAP request
    async fn handle_request(request: Value, state: &Arc<RwLock<ServerState>>) -> Value {
        let command = request["command"].as_str().unwrap_or("unknown");
        let request_seq = request["seq"].as_u64().unwrap_or(0);

        let mut state_lock = state.write().await;
        state_lock.response_seq += 1;
        let response_seq = state_lock.response_seq;

        match command {
            "initialize" => {
                state_lock.initialized = true;
                state_lock.thread_id = 1; // Фиксированный thread ID

                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "initialize",
                    "body": {
                        "supportsConfigurationDoneRequest": true,
                        "supportsEvaluateForHovers": true,
                        "supportsStepBack": false,
                    }
                })
            }

            "setBreakpoints" => {
                let source = request["arguments"]["source"]["path"]
                    .as_str()
                    .unwrap_or("unknown.rs");

                let breakpoints = request["arguments"]["breakpoints"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|bp| bp["line"].as_u64().map(|l| l as u32))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                // Сохраняем breakpoints
                state_lock
                    .breakpoints
                    .insert(source.to_string(), breakpoints.clone());

                // Формируем response с verified breakpoints
                let verified_bps: Vec<_> = breakpoints
                    .iter()
                    .map(|&line| json!({ "verified": true, "line": line }))
                    .collect();

                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "setBreakpoints",
                    "body": { "breakpoints": verified_bps }
                })
            }

            "launch" => {
                state_lock.launched = true;

                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "launch"
                })
            }

            "continue" => {
                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "continue",
                    "body": { "allThreadsContinued": true }
                })
            }

            "next" | "stepIn" | "stepOut" => {
                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": command
                })
            }

            "stackTrace" => {
                let _thread_id = state_lock.thread_id;

                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "stackTrace",
                    "body": {
                        "stackFrames": [
                            {
                                "id": 1,
                                "name": "main",
                                "source": { "path": "/mock/main.rs" },
                                "line": 10,
                                "column": 1
                            }
                        ],
                        "totalFrames": 1
                    }
                })
            }

            "evaluate" => {
                let expression = request["arguments"]["expression"].as_str().unwrap_or("");

                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": "evaluate",
                    "body": {
                        "result": format!("mock_result({})", expression),
                        "variablesReference": 0
                    }
                })
            }

            "terminate" | "disconnect" => {
                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": command
                })
            }

            _ => {
                // Неизвестная команда - возвращаем generic success
                json!({
                    "seq": response_seq,
                    "type": "response",
                    "request_seq": request_seq,
                    "success": true,
                    "command": command
                })
            }
        }
    }

    /// Отправка DAP сообщения (с заголовком Content-Length)
    async fn send_message(
        socket: &mut tokio::net::TcpStream,
        message: &Value,
    ) -> std::io::Result<()> {
        let body = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        tracing::debug!("Sending DAP response: {}", body);

        socket.write_all(header.as_bytes()).await?;
        socket.write_all(body.as_bytes()).await?;
        socket.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_creation() {
        let server = MockDapServer::new().await.unwrap();
        assert!(server.port() > 0);
    }

    #[tokio::test]
    async fn test_mock_server_address() {
        let server = MockDapServer::new().await.unwrap();
        let addr = server.address();
        assert!(addr.starts_with("127.0.0.1:"));
        assert!(addr.contains(&server.port().to_string()));
    }
}
