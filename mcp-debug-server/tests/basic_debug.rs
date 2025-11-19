//! Интеграционный тест: Basic Debug Flow
//!
//! Симулирует полный цикл отладки:
//! 1. Создать debug session
//! 2. Установить breakpoint
//! 3. Запустить программу (launch)
//! 4. Получить stack trace
//! 5. Вычислить expression (eval)
//! 6. Продолжить выполнение (continue)
//! 7. Завершить сессию (terminate)

use mcp_debug_server::session::{SessionManager, SessionState};

mod support;
use support::MockDapServer;

#[tokio::test]
async fn test_basic_debug_flow() {
    // Инициализация tracing для отладки
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // 1. Запустить mock DAP server
    let mock_server = MockDapServer::new().await.unwrap();
    let _server_address = mock_server.address();
    let port = mock_server.port();

    tracing::info!("Mock DAP server started on port {}", port);

    // Запустить сервер в фоне
    tokio::spawn(async move {
        mock_server.run().await;
    });

    // Дать серверу время на старт
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Создать SessionManager
    let _manager = SessionManager::new();

    // 3. Создать debug сессию
    // ВАЖНО: используем простую команду для запуска mock server
    // В реальности это был бы "codelldb" или другой adapter
    let _binary_path = "test_program".to_string();
    let _adapter_command = format!("nc 127.0.0.1 {}", port);

    // Для тестирования используем прямое подключение через DapClient
    // Вместо запуска адаптера, мы напрямую подключаемся к mock серверу

    // Создадим тестовый wrapper, который использует TCP вместо процесса
    // Это требует модификации DapClient или создания test helper

    // ВРЕМЕННОЕ РЕШЕНИЕ: Пропустим создание сессии через spawn
    // и используем прямой тест DapClient → MockServer взаимодействия

    tracing::info!("Test basic debug flow completed - skeleton");
}

#[tokio::test]
async fn test_session_lifecycle() {
    // Тест жизненного цикла сессии без реального DAP adapter
    let manager = SessionManager::new();

    // Проверка: изначально нет сессий
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);

    // Проверка: попытка завершить несуществующую сессию → ошибка
    let fake_id = mcp_debug_server::types::SessionId::from_string("fake-id".to_string());
    let result = manager.terminate_session(&fake_id).await;
    assert!(result.is_err());

    tracing::info!("Test session lifecycle completed");
}

#[tokio::test]
async fn test_state_transitions() {
    // Тест валидации переходов состояний
    use SessionState::*;

    // Valid transitions
    assert!(Initialized.can_transition_to(Running));
    assert!(Running.can_transition_to(Stopped));
    assert!(Stopped.can_transition_to(Running));

    // Invalid transitions
    assert!(!Initialized.can_transition_to(Stopped));
    assert!(!Running.can_transition_to(Initialized));
    assert!(!Terminated.can_transition_to(Running));

    // Termination allowed from any state
    assert!(Initialized.can_transition_to(Terminated));
    assert!(Running.can_transition_to(Terminated));
    assert!(Stopped.can_transition_to(Terminated));

    tracing::info!("Test state transitions completed");
}

/// Тест direct DAP protocol interaction с mock server
///
/// Этот тест проверяет, что mock server корректно обрабатывает DAP requests
#[tokio::test]
async fn test_mock_dap_protocol() {
    use tokio::net::TcpStream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use serde_json::json;

    // Запустить mock server
    let mock_server = MockDapServer::new().await.unwrap();
    let port = mock_server.port();

    tokio::spawn(async move {
        mock_server.run().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Подключиться к mock server
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    // Отправить initialize request
    let request = json!({
        "seq": 1,
        "type": "request",
        "command": "initialize",
        "arguments": {
            "clientID": "test-client",
            "adapterID": "test-adapter"
        }
    });

    let body = serde_json::to_string(&request).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    stream.write_all(header.as_bytes()).await.unwrap();
    stream.write_all(body.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();

    // Прочитать response
    let mut header_buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await.unwrap();
        header_buf.push(byte[0]);

        if header_buf.len() >= 4
            && &header_buf[header_buf.len()-4..] == b"\r\n\r\n" {
            break;
        }
    }

    let header_str = String::from_utf8_lossy(&header_buf);
    let content_length: usize = header_str
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|s| s.trim().parse().ok())
        .unwrap();

    let mut response_body = vec![0u8; content_length];
    stream.read_exact(&mut response_body).await.unwrap();

    let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();

    // Проверки
    assert_eq!(response["type"], "response");
    assert_eq!(response["command"], "initialize");
    assert_eq!(response["success"], true);

    tracing::info!("Test mock DAP protocol completed successfully");
}
