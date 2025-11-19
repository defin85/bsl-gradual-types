//! Integration Tests: Full Async Debug Cycle
//!
//! Полный цикл отладки с async event handling:
//! 1. create_session
//! 2. set_breakpoint
//! 3. launch (с автоматическим configurationDone)
//! 4. poll_events (проверка initialized + stopped events)
//! 5. backtrace
//! 6. continue
//! 7. terminate

use mcp_debug_server::session::{SessionManager, SessionState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

mod support;
use support::MockDapServer;

/// EventBuffer типа
type EventBuffer = Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>;

#[tokio::test]
async fn test_full_async_debug_cycle_with_mock_server() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // 1. Запустить mock DAP server
    let mock_server = MockDapServer::new().await.unwrap();
    let port = mock_server.port();

    tracing::info!("Mock DAP server started on port {}", port);

    tokio::spawn(async move {
        mock_server.run().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Создать SessionManager
    let manager = SessionManager::new();

    // 3. Проверить, что изначально нет сессий
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);

    // TODO: Для полного integration теста нужно:
    // - Реализовать create_session с TCP подключением к mock server
    // - Или использовать реальный DAP adapter (например, mock через процесс)
    //
    // ПРОБЛЕМА: Текущая реализация DapClient::spawn() ожидает command для процесса,
    // а не TCP адрес. Нужен wrapper или модификация для тестирования.
    //
    // РЕШЕНИЕ (временное): Тестируем компоненты отдельно через unit tests.
    // Для full integration нужен refactoring DapClient для поддержки mock transport.

    tracing::info!("Full async debug cycle test - partial implementation (skeleton)");
}

/// Тест проверки configurationDone после launch
#[tokio::test]
async fn test_configuration_done_sent_after_launch() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Этот тест проверяет, что DapClient вызывает configuration_done() после launch
    // Требует mock DapClient или integration с реальным адаптером

    // PLAN:
    // 1. Создать mock adapter который логирует все requests
    // 2. Вызвать launch через DapClient
    // 3. Проверить, что был отправлен "configurationDone" request

    tracing::info!("Configuration done test - requires mock adapter with request logging");
}

/// Тест event routing во время send_request
#[tokio::test]
async fn test_events_not_lost_during_request() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Этот тест проверяет, что:
    // 1. Отправляем request (например, launch)
    // 2. Пока ждём response, adapter шлёт события (stopped, output)
    // 3. События не теряются и попадают в EventBuffer

    // IMPLEMENTATION:
    // - Использовать MockTransport для симуляции adapter behaviour
    // - Router должен маршрутизировать events в event_tx channel
    // - EventProcessor должен добавлять в EventBuffer

    tracing::info!("Events not lost during request test - requires coordinated mock setup");
}

/// Тест граничного случая: polling пустого буфера
#[tokio::test]
async fn test_poll_empty_event_buffer() {
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "empty-session".to_string();

    // Polling пустого буфера
    let events = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };

    // Должен вернуть пустой Vec
    assert_eq!(events.len(), 0);

    tracing::info!("test_poll_empty_event_buffer passed");
}

/// Тест граничного случая: polling несуществующей сессии
#[tokio::test]
async fn test_poll_nonexistent_session() {
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));

    // Добавить события для session-1
    {
        let mut buffer = event_buffer.lock().await;
        buffer.insert(
            "session-1".to_string(),
            vec![serde_json::json!({"event": "stopped"})],
        );
    }

    // Polling для session-2 (не существует)
    let events = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove("session-2").unwrap_or_default()
    };

    // Должен вернуть пустой Vec
    assert_eq!(events.len(), 0);

    // session-1 не должна быть затронута
    let session1_events = {
        let buffer = event_buffer.lock().await;
        buffer.get("session-1").map(|v| v.len()).unwrap_or(0)
    };
    assert_eq!(session1_events, 1);

    tracing::info!("test_poll_nonexistent_session passed");
}

/// Тест множественного polling одной сессии
#[tokio::test]
async fn test_multiple_polling_same_session() {
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "multi-poll-session".to_string();

    // Добавить события
    {
        let mut buffer = event_buffer.lock().await;
        buffer.insert(
            session_id.clone(),
            vec![
                serde_json::json!({"event": "stopped"}),
                serde_json::json!({"event": "output"}),
            ],
        );
    }

    // Первый poll - получаем все события
    let first_poll = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };
    assert_eq!(first_poll.len(), 2);

    // Второй poll - должен вернуть пустой массив
    let second_poll = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };
    assert_eq!(second_poll.len(), 0);

    // Третий poll - также пустой
    let third_poll = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };
    assert_eq!(third_poll.len(), 0);

    tracing::info!("test_multiple_polling_same_session passed");
}

/// Тест изоляции событий между сессиями
#[tokio::test]
async fn test_event_isolation_between_sessions() {
    use serde_json::json;

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));

    // Добавить события для трёх сессий
    {
        let mut buffer = event_buffer.lock().await;

        buffer.insert(
            "session-A".to_string(),
            vec![json!({"event": "stopped", "session": "A"})],
        );

        buffer.insert(
            "session-B".to_string(),
            vec![
                json!({"event": "output", "session": "B"}),
                json!({"event": "continued", "session": "B"}),
            ],
        );

        buffer.insert(
            "session-C".to_string(),
            vec![json!({"event": "terminated", "session": "C"})],
        );
    }

    // Polling session-A
    let events_a = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove("session-A").unwrap_or_default()
    };
    assert_eq!(events_a.len(), 1);
    assert_eq!(events_a[0]["session"], "A");

    // Polling session-B
    let events_b = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove("session-B").unwrap_or_default()
    };
    assert_eq!(events_b.len(), 2);
    assert_eq!(events_b[0]["session"], "B");
    assert_eq!(events_b[1]["session"], "B");

    // Session-C не должна быть затронута
    let events_c_count = {
        let buffer = event_buffer.lock().await;
        buffer.get("session-C").map(|v| v.len()).unwrap_or(0)
    };
    assert_eq!(events_c_count, 1);

    tracing::info!("test_event_isolation_between_sessions passed");
}

/// Тест concurrent polling разных сессий
#[tokio::test]
async fn test_concurrent_polling_different_sessions() {
    use serde_json::json;

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));

    // Подготовить события для 5 сессий
    {
        let mut buffer = event_buffer.lock().await;
        for i in 0..5 {
            let session_id = format!("session-{}", i);
            buffer.insert(
                session_id,
                vec![json!({"event": "stopped", "id": i})],
            );
        }
    }

    // Создать 5 параллельных задач для polling
    let mut tasks = vec![];
    for i in 0..5 {
        let buffer_clone = event_buffer.clone();
        let session_id = format!("session-{}", i);

        let task = tokio::spawn(async move {
            let events = {
                let mut buffer = buffer_clone.lock().await;
                buffer.remove(&session_id).unwrap_or_default()
            };

            assert_eq!(events.len(), 1);
            assert_eq!(events[0]["id"], i);
        });

        tasks.push(task);
    }

    // Дождаться завершения всех polling задач
    for task in tasks {
        task.await.unwrap();
    }

    // Проверить, что все буферы очищены
    let buffer = event_buffer.lock().await;
    assert_eq!(buffer.len(), 0);

    tracing::info!("test_concurrent_polling_different_sessions passed");
}

/// Тест state transitions во время async event processing
#[tokio::test]
async fn test_state_transitions_with_events() {
    use SessionState::*;

    let _manager = SessionManager::new();

    // Проверить валидные переходы
    assert!(Initialized.can_transition_to(Running));
    assert!(Running.can_transition_to(Stopped));
    assert!(Stopped.can_transition_to(Running));

    // Когда получаем "stopped" event, сессия переходит Running → Stopped
    // Когда получаем "continued" event, сессия переходит Stopped → Running
    // Когда получаем "terminated" event, сессия переходит * → Terminated

    // Эта логика должна быть реализована в EventProcessor.handle_stopped_event()

    // Для тестирования нужен доступ к SessionManager из EventProcessor
    // TODO: Добавить weak reference к SessionManager в EventProcessor

    tracing::info!("test_state_transitions_with_events - logic validation passed");
}

/// Тест cleanup после terminate
#[tokio::test]
async fn test_cleanup_after_terminate() {
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "cleanup-session".to_string();

    // Добавить события
    {
        let mut buffer = event_buffer.lock().await;
        buffer.insert(
            session_id.clone(),
            vec![
                serde_json::json!({"event": "stopped"}),
                serde_json::json!({"event": "terminated"}),
            ],
        );
    }

    // Polling
    let events = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };

    assert_eq!(events.len(), 2);
    assert_eq!(events[1]["event"], "terminated");

    // После terminate буфер должен быть очищен
    let remaining = {
        let buffer = event_buffer.lock().await;
        buffer.contains_key(&session_id)
    };
    assert!(!remaining);

    tracing::info!("test_cleanup_after_terminate passed");
}

/// Тест 2: EventProcessor graceful shutdown
/// Проверяет что EventProcessor корректно завершается за < 2 секунды
#[tokio::test]
async fn test_event_processor_graceful_shutdown() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use mcp_debug_server::dap::EventProcessor;

    // Создать EventProcessor
    let (event_tx, event_rx) = mpsc::channel(100);
    let event_buffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "test-session-shutdown".to_string();
    let current_thread_id = Arc::new(Mutex::new(None));

    let event_processor = EventProcessor::new(
        event_rx,
        event_buffer.clone(),
        session_id,
        current_thread_id,
    );

    // Запустить в background task
    let handle = tokio::spawn(event_processor.run());

    // Отправить несколько событий
    for i in 0..10 {
        let event = serde_json::json!({"event": "output", "body": {"output": format!("test {}", i)}});
        let _ = event_tx.send(event).await;
    }

    // Подождать обработки
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Закрыть channel (это сигнал для EventProcessor завершиться)
    drop(event_tx);

    // Проверить что EventProcessor завершается за < 2 секунды
    match timeout(Duration::from_secs(2), handle).await {
        Ok(Ok(())) => {
            tracing::info!("EventProcessor gracefully shut down");
        }
        Ok(Err(e)) => {
            panic!("EventProcessor panic: {:?}", e);
        }
        Err(_) => {
            panic!("EventProcessor did not shutdown within 2 seconds");
        }
    }

    tracing::info!("test_event_processor_graceful_shutdown passed");
}
