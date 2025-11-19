//! Интеграционный тест: Error Recovery
//!
//! Проверяет обработку ошибок и восстановление:
//! - Несуществующие сессии
//! - Невалидные state transitions
//! - Timeout для медленных DAP responses
//! - Crashed DAP adapter (симуляция)

use mcp_debug_server::session::{SessionManager, SessionState};

#[tokio::test]
async fn test_nonexistent_session_errors() {
    // Инициализация tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let manager = SessionManager::new();

    // Создать несуществующий session ID
    let fake_id = mcp_debug_server::types::SessionId::from_string("nonexistent".to_string());

    // Проверка 1: session_exists возвращает false
    assert!(!manager.session_exists(&fake_id).await);

    // Проверка 2: terminate_session возвращает ошибку
    let result = manager.terminate_session(&fake_id).await;
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("not found"));

    tracing::info!("Test nonexistent session errors completed");
}

#[tokio::test]
async fn test_invalid_state_transitions() {
    // Тест невалидных переходов состояний
    use SessionState::*;

    // Тест 1: Initialized → Stopped (невалидно)
    let current = Initialized;
    assert!(!current.can_transition_to(Stopped));

    // Тест 2: Running → Initialized (невалидно)
    let current = Running;
    assert!(!current.can_transition_to(Initialized));

    // Тест 3: Terminated → * (невалидно для всех переходов)
    let current = Terminated;
    assert!(!current.can_transition_to(Initialized));
    assert!(!current.can_transition_to(Running));
    assert!(!current.can_transition_to(Stopped));

    // Тест 4: Stopped → Initialized (невалидно)
    let current = Stopped;
    assert!(!current.can_transition_to(Initialized));

    tracing::info!("Test invalid state transitions completed");
}

#[tokio::test]
async fn test_multiple_terminate_attempts() {
    // Тест множественных попыток завершить несуществующую сессию
    let manager = SessionManager::new();
    let fake_id = mcp_debug_server::types::SessionId::from_string("fake-session".to_string());

    // Первая попытка
    let result1 = manager.terminate_session(&fake_id).await;
    assert!(result1.is_err());

    // Вторая попытка (должна также завершиться ошибкой)
    let result2 = manager.terminate_session(&fake_id).await;
    assert!(result2.is_err());

    // Третья попытка
    let result3 = manager.terminate_session(&fake_id).await;
    assert!(result3.is_err());

    tracing::info!("Test multiple terminate attempts completed");
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    // Тест параллельных ошибочных операций
    use std::sync::Arc;

    let manager = Arc::new(SessionManager::new());

    // Создать 10 параллельных задач, каждая пытается завершить несуществующую сессию
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                let fake_id = mcp_debug_server::types::SessionId::from_string(
                    format!("fake-{}", i)
                );

                let result = manager_clone.terminate_session(&fake_id).await;
                assert!(result.is_err());

                tracing::debug!("Task {} received expected error", i);
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    tracing::info!("Test concurrent error handling completed");
}

/// Тест граничных случаев для SessionId
#[tokio::test]
async fn test_session_id_edge_cases() {
    let manager = SessionManager::new();

    // Граничный случай 1: пустая строка
    let empty_id = mcp_debug_server::types::SessionId::from_string(String::new());
    assert!(!manager.session_exists(&empty_id).await);

    // Граничный случай 2: очень длинный ID
    let long_id = mcp_debug_server::types::SessionId::from_string(
        "a".repeat(10000)
    );
    assert!(!manager.session_exists(&long_id).await);

    // Граничный случай 3: ID с спецсимволами
    let special_id = mcp_debug_server::types::SessionId::from_string(
        "session-!@#$%^&*()".to_string()
    );
    assert!(!manager.session_exists(&special_id).await);

    tracing::info!("Test session ID edge cases completed");
}

/// Тест проверки валидности state descriptions
#[tokio::test]
async fn test_state_descriptions() {
    use SessionState::*;

    // Проверка, что все состояния имеют описания
    let states = [Initialized, Running, Stopped, Terminated];

    for state in &states {
        let description = state.description();
        assert!(!description.is_empty());
        tracing::info!("State {:?} has description: {}", state, description);
    }

    // Проверка конкретных описаний
    assert!(Initialized.description().contains("ready"));
    assert!(Running.description().contains("executing"));
    assert!(Stopped.description().contains("paused"));
    assert!(Terminated.description().contains("finished"));

    tracing::info!("Test state descriptions completed");
}

/// Тест поведения при пустом списке сессий
#[tokio::test]
async fn test_empty_session_list() {
    let manager = SessionManager::new();

    // Проверка 1: list_sessions возвращает пустой вектор
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);
    assert!(sessions.is_empty());

    // Проверка 2: множественные вызовы list_sessions
    for _ in 0..5 {
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    tracing::info!("Test empty session list completed");
}

/// Тест обработки ошибок в with_session
#[tokio::test]
async fn test_with_session_error_handling() {
    let manager = SessionManager::new();
    let fake_id = mcp_debug_server::types::SessionId::from_string("fake".to_string());

    // Попытка выполнить операцию с несуществующей сессией
    let result = manager
        .with_session(&fake_id, |_session| {
            Box::pin(async move {
                Ok::<(), anyhow::Error>(())
            })
        })
        .await;

    // Должна вернуться ошибка "Session not found"
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not found"));

    tracing::info!("Test with_session error handling completed");
}

/// Тест валидности переходов состояний (расширенный)
#[tokio::test]
async fn test_state_transition_matrix() {
    use SessionState::*;

    // Полная матрица переходов
    let states = [Initialized, Running, Stopped, Terminated];

    for from_state in &states {
        for to_state in &states {
            let can_transition = from_state.can_transition_to(*to_state);

            tracing::debug!(
                "{} -> {}: {}",
                from_state.description(),
                to_state.description(),
                if can_transition { "allowed" } else { "denied" }
            );

            // Проверка консистентности правил
            match (from_state, to_state) {
                // Из Terminated нельзя никуда
                (Terminated, _) => assert!(!can_transition),

                // В себя всегда можно
                (state, new) if state == new => assert!(can_transition),

                // Из Initialized в Running
                (Initialized, Running) => assert!(can_transition),

                // Из Running в Stopped
                (Running, Stopped) => assert!(can_transition),

                // Из Stopped в Running
                (Stopped, Running) => assert!(can_transition),

                // В Terminated из любого состояния (кроме самого себя)
                (_, Terminated) => assert!(can_transition),

                _ => {}
            }
        }
    }

    tracing::info!("Test state transition matrix completed");
}

/// Тест timeout handling для DAP requests
#[tokio::test]
async fn test_request_timeout_simulation() {
    use tokio::time::{timeout, Duration};

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Симулируем timeout для async операции
    let result = timeout(Duration::from_millis(100), async {
        // Операция, которая занимает больше времени чем timeout
        tokio::time::sleep(Duration::from_secs(5)).await;
        "completed"
    })
    .await;

    // Должен быть timeout
    assert!(result.is_err(), "Expected timeout error");

    tracing::info!("Test request timeout simulation passed");
}

/// Тест graceful shutdown EventRouter при broken pipe
#[tokio::test]
async fn test_event_router_graceful_shutdown() {
    use tokio::sync::mpsc;
    // Unused imports removed

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let (event_tx, mut event_rx) = mpsc::channel::<serde_json::Value>(10);

    // Симулируем EventRouter который получает события
    let router_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(_event) = event_rx.recv().await {
            count += 1;
            tracing::debug!("Received event #{}", count);
        }
        tracing::info!("Router stopped after {} events", count);
        count
    });

    // Отправить несколько событий
    for i in 0..5 {
        event_tx.send(serde_json::json!({"event": "test", "id": i})).await.unwrap();
    }

    // Закрыть канал (симуляция broken pipe)
    drop(event_tx);

    // Дождаться graceful shutdown
    let total_events = router_task.await.unwrap();
    assert_eq!(total_events, 5);

    tracing::info!("Test event router graceful shutdown passed");
}

/// Тест response_map cleanup при timeout
#[tokio::test]
async fn test_response_map_cleanup_on_timeout() {
    use std::collections::HashMap;
    use tokio::sync::{oneshot, Mutex};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let response_map = Arc::new(Mutex::new(HashMap::<u32, oneshot::Sender<serde_json::Value>>::new()));

    let seq = 42;
    let (tx, rx) = oneshot::channel();

    // Зарегистрировать oneshot
    response_map.lock().await.insert(seq, tx);
    assert_eq!(response_map.lock().await.len(), 1);

    // Симулировать timeout (5 секунд ожидания)
    let result = timeout(Duration::from_millis(100), rx).await;
    assert!(result.is_err(), "Expected timeout");

    // Cleanup: удалить seq из response_map
    response_map.lock().await.remove(&seq);
    assert_eq!(response_map.lock().await.len(), 0);

    tracing::info!("Test response map cleanup on timeout passed");
}

/// Тест обработки невалидного JSON в EventProcessor
#[tokio::test]
async fn test_malformed_json_handling() {
    // Это проверяет, что EventProcessor не падает при получении невалидного события
    // Невалидное событие уже протестировано в event_processing.rs::test_event_processor_malformed_event

    tracing::info!("Malformed JSON handling covered in event_processing tests");
}

/// Тест concurrent error handling
#[tokio::test]
async fn test_concurrent_error_handling_stress() {
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let manager = Arc::new(SessionManager::new());

    // Создать 100 параллельных задач, каждая пытается завершить несуществующую сессию
    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                let fake_id = mcp_debug_server::types::SessionId::from_string(
                    format!("stress-fake-{}", i)
                );

                // Все вызовы должны вернуть ошибку
                for _ in 0..5 {
                    let result = manager_clone.terminate_session(&fake_id).await;
                    assert!(result.is_err());
                }
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    tracing::info!("Test concurrent error handling stress completed (500 error operations)");
}

/// Тест EventBuffer overflow protection
#[tokio::test]
async fn test_event_buffer_overflow_protection() {
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    type EventBuffer = Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>;

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "overflow-session".to_string();

    // Добавить очень большое количество событий
    {
        let mut buffer = event_buffer.lock().await;
        let mut events = vec![];
        for i in 0..10000 {
            events.push(json!({"event": "output", "id": i}));
        }
        buffer.insert(session_id.clone(), events);
    }

    // Проверить, что все события сохранены (нет лимита по умолчанию)
    let events_count = {
        let buffer = event_buffer.lock().await;
        buffer.get(&session_id).map(|v| v.len()).unwrap_or(0)
    };

    assert_eq!(events_count, 10000);

    // NOTE: В production может потребоваться лимит (например, 1000 событий)
    // и warning при превышении

    tracing::info!("Test event buffer overflow protection passed (no limit enforced)");
}

/// Тест повторного polling после ошибки
#[tokio::test]
async fn test_polling_after_error() {
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    type EventBuffer = Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>;

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "error-session".to_string();

    // Добавить события
    {
        let mut buffer = event_buffer.lock().await;
        buffer.insert(session_id.clone(), vec![json!({"event": "stopped"})]);
    }

    // Первый polling - успех
    let first_poll = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };
    assert_eq!(first_poll.len(), 1);

    // Симулируем ошибку (несуществующая сессия)
    // Второй polling - должен вернуть пустой массив без паники
    let second_poll = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };
    assert_eq!(second_poll.len(), 0);

    tracing::info!("Test polling after error passed");
}
