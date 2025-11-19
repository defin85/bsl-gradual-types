use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use serde_json::{json, Value};
use crate::types::{DapResult, DapError};
use super::transport::{DapTransport, DapWriter};
use super::protocol::{DapRequest, DapResponse};
use super::router::EventRouter;

pub struct DapClient {
    process: Child,
    writer: DapWriter,
    seq_counter: u32,
    response_map: Arc<Mutex<HashMap<u32, oneshot::Sender<Value>>>>,
}

impl DapClient {
    /// Запустить DAP adapter (например, CodeLLDB) и вернуть (DapClient, event_rx)
    #[tracing::instrument]
    pub async fn spawn(adapter_command: &str) -> DapResult<(Self, mpsc::Receiver<Value>)> {
        tracing::info!("Spawning DAP adapter process");
        tracing::debug!("Adapter command: {}", adapter_command);

        // Установить рабочую директорию адаптера, если путь абсолютный
        // Это нужно для lldb-dap.exe, чтобы он мог найти свои DLL зависимости
        let mut command = Command::new(adapter_command);

        if let Some(parent_dir) = std::path::Path::new(adapter_command).parent() {
            if parent_dir.as_os_str().len() > 0 {
                tracing::debug!("Setting adapter working directory: {:?}", parent_dir);
                command.current_dir(parent_dir);
            }
        }

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                tracing::error!("Failed to spawn DAP adapter: {}", e);
                crate::types::DapError::SpawnFailed(e.to_string())
            })?;

        let stdin = child.stdin.take().expect("Failed to get stdin");
        let stdout = child.stdout.take().expect("Failed to get stdout");

        let transport = DapTransport::new(stdin, stdout);
        let (writer, reader) = transport.split();

        // Создать каналы для событий и responses
        let (event_tx, event_rx) = mpsc::channel(100);
        let response_map = Arc::new(Mutex::new(HashMap::new()));

        // Запустить EventRouter в background task
        let router = EventRouter::new(reader, event_tx, response_map.clone());
        tokio::spawn(router.run());

        tracing::info!("DAP adapter process spawned successfully");

        Ok((
            Self {
                process: child,
                writer,
                seq_counter: 1,
                response_map,
            },
            event_rx,
        ))
    }

    /// Отправить DAP initialize request
    #[tracing::instrument(skip(self))]
    pub async fn initialize(&mut self) -> DapResult<Value> {
        tracing::debug!("Initializing DAP session");
        self.send_request("initialize", Some(json!({
            "clientID": "mcp-debug-server",
            "adapterID": "lldb",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "pathFormat": "path",
        }))).await
    }

    /// Установить breakpoint
    pub async fn set_breakpoints(&mut self, file: &str, lines: &[u32]) -> DapResult<Value> {
        let breakpoints: Vec<_> = lines.iter().map(|line| json!({"line": line})).collect();
        
        self.send_request("setBreakpoints", Some(json!({
            "source": { "path": file },
            "breakpoints": breakpoints,
        }))).await
    }

    /// Запустить программу (с ожиданием ответа)
    ///
    /// ВАЖНО: По DAP спецификации, launch response приходит ТОЛЬКО ПОСЛЕ:
    /// 1. Получения initialized event
    /// 2. Отправки configurationDone
    ///
    /// Используйте launch_and_configure() для правильной последовательности!
    pub async fn launch(&mut self, program: &str, args: Option<Vec<String>>) -> DapResult<Value> {
        self.send_request("launch", Some(json!({
            "program": program,
            "args": args.unwrap_or_default(),
            "stopOnEntry": true,
        }))).await
    }

    /// Запустить программу БЕЗ ожидания ответа
    ///
    /// Используется для правильной DAP последовательности:
    /// 1. launch_no_wait()
    /// 2. Дождаться initialized event (через EventBuffer)
    /// 3. configuration_done()
    pub async fn launch_no_wait(&mut self, program: &str, args: Option<Vec<String>>) -> DapResult<()> {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        let request = DapRequest {
            seq,
            type_: "request".to_string(),
            command: "launch".to_string(),
            arguments: Some(json!({
                "program": program,
                "args": args.unwrap_or_default(),
                "stopOnEntry": true,
            })),
        };

        let request_json = serde_json::to_value(&request)?;
        self.writer.send(&request_json).await?;

        tracing::debug!("Sent launch request (no wait for response)");
        Ok(())
    }

    /// Продолжить выполнение
    pub async fn continue_execution(&mut self, thread_id: u32) -> DapResult<Value> {
        self.send_request("continue", Some(json!({
            "threadId": thread_id,
        }))).await
    }

    /// Step over (next line)
    pub async fn next(&mut self, thread_id: u32) -> DapResult<Value> {
        self.send_request("next", Some(json!({
            "threadId": thread_id,
        }))).await
    }

    /// Step into
    pub async fn step_in(&mut self, thread_id: u32) -> DapResult<Value> {
        self.send_request("stepIn", Some(json!({
            "threadId": thread_id,
        }))).await
    }

    /// Step out of current function
    pub async fn step_out(&mut self, thread_id: u32) -> DapResult<Value> {
        self.send_request("stepOut", Some(json!({
            "threadId": thread_id,
        }))).await
    }

    /// Получить stack trace
    pub async fn stack_trace(&mut self, thread_id: u32) -> DapResult<Value> {
        self.send_request("stackTrace", Some(json!({
            "threadId": thread_id,
        }))).await
    }

    /// Завершить debug сессию
    pub async fn terminate(&mut self) -> DapResult<Value> {
        self.send_request("terminate", None).await
    }

    /// Вычислить expression
    pub async fn evaluate(&mut self, expression: &str, frame_id: Option<u32>) -> DapResult<Value> {
        // Используем context "watch" для просмотра переменных
        // "repl" может интерпретировать как LLDB команды, что вызывает ошибки
        let mut args = json!({
            "expression": expression,
            "context": "watch",
        });

        // frameId обязателен для evaluate
        if let Some(fid) = frame_id {
            args["frameId"] = json!(fid);
        }

        self.send_request("evaluate", Some(args)).await
    }

    /// Отправить generic request и дождаться response (с timeout)
    #[tracing::instrument(skip(self, arguments))]
    async fn send_request(&mut self, command: &str, arguments: Option<Value>) -> DapResult<Value> {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        tracing::debug!(command = %command, seq = %seq, "Sending DAP request");

        let request = DapRequest {
            seq,
            type_: "request".to_string(),
            command: command.to_string(),
            arguments,
        };

        // Создать oneshot channel для response
        let (tx, rx) = oneshot::channel();

        // Зарегистрировать ожидание response
        self.response_map.lock().await.insert(seq, tx);

        // Отправить request
        let request_json = serde_json::to_value(&request)?;
        self.writer.send(&request_json).await?;

        // Ждать response через oneshot с timeout (5 секунд)
        match timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(message)) => {
                // Обработать response
                let response: DapResponse = serde_json::from_value(message)?;

                if !response.success {
                    let error_msg = response.message.unwrap_or_else(|| "Unknown error".to_string());
                    tracing::error!(command = %command, error = %error_msg, "DAP request failed");
                    return Err(DapError::Protocol(error_msg));
                }

                tracing::debug!(command = %command, "DAP request completed successfully");
                Ok(response.body.unwrap_or(Value::Null))
            }
            Ok(Err(_)) => {
                tracing::error!(command = %command, "Response channel closed");
                Err(DapError::Protocol("Response channel closed".to_string()))
            }
            Err(_) => {
                // Cleanup на timeout
                self.response_map.lock().await.remove(&seq);
                tracing::error!(command = %command, "DAP request timed out after 5 seconds");
                Err(DapError::Timeout)
            }
        }
    }

    /// Отправить configurationDone request (обязательно после launch)
    pub async fn configuration_done(&mut self) -> DapResult<Value> {
        self.send_request("configurationDone", None).await
    }
}

impl Drop for DapClient {
    fn drop(&mut self) {
        // Попытка graceful shutdown
        let _ = self.process.start_kill();
    }
}
