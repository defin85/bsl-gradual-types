//! Mock Transport для unit-тестирования DAP компонентов
//!
//! Предоставляет in-memory каналы для эмуляции stdin/stdout

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

/// Mock stdin/stdout пара для тестирования
pub struct MockTransport {
    /// Client-side stdin (для записи)
    pub client_stdin: DuplexStream,
    /// Client-side stdout (для чтения)
    pub client_stdout: DuplexStream,
    /// Server-side stdin (для чтения запросов)
    pub server_stdin: DuplexStream,
    /// Server-side stdout (для записи responses)
    pub server_stdout: DuplexStream,
}

impl MockTransport {
    /// Создать пару in-memory каналов
    pub fn new() -> Self {
        let (client_stdin, server_stdin) = tokio::io::duplex(8192);
        let (server_stdout, client_stdout) = tokio::io::duplex(8192);

        Self {
            client_stdin,
            client_stdout,
            server_stdin,
            server_stdout,
        }
    }

    /// Отправить JSON сообщение с DAP protocol headers
    pub async fn send_message(stream: &mut DuplexStream, message: &Value) -> std::io::Result<()> {
        let body = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        stream.write_all(header.as_bytes()).await?;
        stream.write_all(body.as_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }

    /// Получить JSON сообщение с DAP protocol headers
    pub async fn receive_message(stream: &mut DuplexStream) -> std::io::Result<Value> {
        // Читаем заголовок Content-Length
        let mut header_buf = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            stream.read_exact(&mut byte).await?;
            header_buf.push(byte[0]);

            if header_buf.len() >= 4 && &header_buf[header_buf.len() - 4..] == b"\r\n\r\n" {
                break;
            }
        }

        let header_str = String::from_utf8_lossy(&header_buf);
        let content_length: usize = header_str
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Content-Length")
            })?;

        // Читаем тело
        let mut body = vec![0u8; content_length];
        stream.read_exact(&mut body).await?;

        serde_json::from_slice(&body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mock_transport_creation() {
        let _transport = MockTransport::new();
        // Просто проверяем, что создание не падает
    }

    #[tokio::test]
    async fn test_send_receive_message() {
        let mut transport = MockTransport::new();

        let test_message = json!({
            "seq": 1,
            "type": "request",
            "command": "initialize"
        });

        // Отправить через client_stdin
        MockTransport::send_message(&mut transport.client_stdin, &test_message)
            .await
            .unwrap();

        // Получить через server_stdin
        let received = MockTransport::receive_message(&mut transport.server_stdin)
            .await
            .unwrap();

        assert_eq!(received, test_message);
    }

    #[tokio::test]
    async fn test_bidirectional_communication() {
        let mut transport = MockTransport::new();

        // Client → Server
        let request = json!({"seq": 1, "type": "request", "command": "test"});
        MockTransport::send_message(&mut transport.client_stdin, &request)
            .await
            .unwrap();

        let received_request = MockTransport::receive_message(&mut transport.server_stdin)
            .await
            .unwrap();
        assert_eq!(received_request, request);

        // Server → Client
        let response = json!({"seq": 1, "type": "response", "success": true});
        MockTransport::send_message(&mut transport.server_stdout, &response)
            .await
            .unwrap();

        let received_response = MockTransport::receive_message(&mut transport.client_stdout)
            .await
            .unwrap();
        assert_eq!(received_response, response);
    }
}
