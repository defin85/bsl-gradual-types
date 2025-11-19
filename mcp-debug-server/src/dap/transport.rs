use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::types::DapResult;
use serde_json::Value;

/// DAP Transport Writer - отвечает за отправку сообщений
pub struct DapWriter {
    stdin: Arc<Mutex<ChildStdin>>,
}

impl DapWriter {
    /// Отправить JSON сообщение
    pub async fn send(&self, message: &Value) -> DapResult<()> {
        let body = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(body.as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }
}

/// DAP Transport Reader - отвечает за чтение сообщений
pub struct DapReader {
    stdout: BufReader<ChildStdout>,
}

impl DapReader {
    /// Получить JSON сообщение
    pub async fn receive(&mut self) -> DapResult<Value> {
        // Читаем Content-Length header
        let mut header_line = String::new();
        self.stdout.read_line(&mut header_line).await?;

        if !header_line.starts_with("Content-Length:") {
            return Err(crate::types::DapError::Protocol(
                format!("Expected Content-Length header, got: {}", header_line)
            ));
        }

        let length: usize = header_line
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()
            .map_err(|e| crate::types::DapError::Protocol(format!("Invalid Content-Length: {}", e)))?;

        // Пропускаем пустую строку после headers
        let mut empty_line = String::new();
        self.stdout.read_line(&mut empty_line).await?;

        // Читаем JSON body
        let mut buffer = vec![0u8; length];
        self.stdout.read_exact(&mut buffer).await?;

        let message: Value = serde_json::from_slice(&buffer)?;
        Ok(message)
    }
}

/// DAP использует Content-Length headers (как LSP)
pub struct DapTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl DapTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    /// Разделить transport на writer и reader для независимой работы
    pub fn split(self) -> (DapWriter, DapReader) {
        let DapTransport { stdin, stdout } = self;

        let writer = DapWriter {
            stdin: Arc::new(Mutex::new(stdin)),
        };

        let reader = DapReader {
            stdout,
        };

        (writer, reader)
    }

    /// Отправить JSON сообщение (legacy, для совместимости)
    ///
    /// Note: This method is not used directly - prefer `split()` to get `DapWriter`.
    #[allow(dead_code)]
    pub async fn send(&mut self, message: &Value) -> DapResult<()> {
        let body = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(body.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    /// Получить JSON сообщение (legacy, для совместимости)
    ///
    /// Note: This method is not used directly - prefer `split()` to get `DapReader`.
    #[allow(dead_code)]
    pub async fn receive(&mut self) -> DapResult<Value> {
        // Читаем Content-Length header
        let mut header_line = String::new();
        self.stdout.read_line(&mut header_line).await?;

        if !header_line.starts_with("Content-Length:") {
            return Err(crate::types::DapError::Protocol(
                format!("Expected Content-Length header, got: {}", header_line)
            ));
        }

        let length: usize = header_line
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()
            .map_err(|e| crate::types::DapError::Protocol(format!("Invalid Content-Length: {}", e)))?;

        // Пропускаем пустую строку после headers
        let mut empty_line = String::new();
        self.stdout.read_line(&mut empty_line).await?;

        // Читаем JSON body
        let mut buffer = vec![0u8; length];
        self.stdout.read_exact(&mut buffer).await?;

        let message: Value = serde_json::from_slice(&buffer)?;
        Ok(message)
    }
}
