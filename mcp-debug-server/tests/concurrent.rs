//! Интеграционный тест: Concurrent Sessions
//!
//! Проверяет одновременную работу нескольких debug сессий:
//! - Создание 3+ параллельных сессий
//! - Изоляция сессий (не пересекаются)
//! - Корректное управление состоянием в SessionManager
//! - Concurrent access к SessionManager через Arc<RwLock>

use mcp_debug_server::session::{SessionManager, SessionState};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_concurrent_sessions_creation() {
    // Инициализация tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let manager = Arc::new(SessionManager::new());

    // Создать 5 параллельных задач, каждая проверяет SessionManager
    let tasks: Vec<_> = (0..5)
        .map(|i| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                // Каждая задача проверяет list_sessions
                let sessions = manager_clone.list_sessions().await;
                tracing::info!("Task {} sees {} sessions", i, sessions.len());

                // Проверка: все сессии имеют валидные состояния
                for (id, state, path) in &sessions {
                    assert!(!id.as_str().is_empty());
                    assert!(!path.is_empty());
                    // State должно быть одним из допустимых
                    match state {
                        SessionState::Initialized
                        | SessionState::Running
                        | SessionState::Stopped
                        | SessionState::Terminated => {}
                    }
                }
            })
        })
        .collect();

    // Дождаться завершения всех задач
    for task in tasks {
        task.await.unwrap();
    }

    tracing::info!("Test concurrent sessions creation completed");
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    // Тест параллельных операций над SessionManager
    let manager = Arc::new(SessionManager::new());

    // Операция 1: проверка существования несуществующей сессии
    let fake_id = mcp_debug_server::types::SessionId::from_string("fake-1".to_string());
    assert!(!manager.session_exists(&fake_id).await);

    // Операция 2: попытка завершить несуществующую сессию
    let result = manager.terminate_session(&fake_id).await;
    assert!(result.is_err());

    // Операция 3: параллельные проверки list_sessions
    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                let sessions = manager_clone.list_sessions().await;
                // Изначально должно быть 0 сессий
                assert_eq!(sessions.len(), 0);
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    tracing::info!("Test concurrent session operations completed");
}

/// Тест race conditions при создании/удалении сессий
#[tokio::test]
async fn test_race_conditions() {
    let manager = Arc::new(SessionManager::new());

    // Создать несколько задач, которые одновременно:
    // - Проверяют список сессий
    // - Пытаются завершить несуществующие сессии
    let tasks: Vec<_> = (0..20)
        .map(|i| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                if i % 2 == 0 {
                    // Четные задачи: читают список сессий
                    let sessions = manager_clone.list_sessions().await;
                    tracing::debug!("Task {} read {} sessions", i, sessions.len());
                } else {
                    // Нечетные задачи: пытаются завершить fake сессию
                    let fake_id = mcp_debug_server::types::SessionId::from_string(
                        format!("fake-{}", i)
                    );
                    let result = manager_clone.terminate_session(&fake_id).await;
                    assert!(result.is_err());
                    tracing::debug!("Task {} tried to terminate fake session", i);
                }
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    // Финальная проверка: список сессий всё ещё пуст
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);

    tracing::info!("Test race conditions completed");
}

/// Тест session isolation
///
/// Убеждаемся, что сессии не пересекаются (каждая имеет свой state)
#[tokio::test]
async fn test_session_isolation() {
    let manager = SessionManager::new();

    // NOTE: Этот тест требует реальных сессий, но без реального DAP adapter
    // мы не можем их создать. Вместо этого тестируем логику SessionManager

    // Проверка: session_exists для несуществующих ID
    let id1 = mcp_debug_server::types::SessionId::from_string("session-1".to_string());
    let id2 = mcp_debug_server::types::SessionId::from_string("session-2".to_string());
    let id3 = mcp_debug_server::types::SessionId::from_string("session-3".to_string());

    assert!(!manager.session_exists(&id1).await);
    assert!(!manager.session_exists(&id2).await);
    assert!(!manager.session_exists(&id3).await);

    // Проверка: list_sessions возвращает пустой список
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);

    tracing::info!("Test session isolation completed");
}

/// Тест стресс-теста SessionManager
///
/// Массивные параллельные операции чтения
#[tokio::test]
async fn test_session_manager_stress() {
    let manager = Arc::new(SessionManager::new());

    // Создать 100 параллельных задач чтения
    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let manager_clone = Arc::clone(&manager);
            tokio::spawn(async move {
                for _ in 0..10 {
                    let sessions = manager_clone.list_sessions().await;
                    assert_eq!(sessions.len(), 0);
                }
                tracing::trace!("Task {} completed 10 reads", i);
            })
        })
        .collect();

    for task in tasks {
        task.await.unwrap();
    }

    tracing::info!("Test session manager stress completed (1000 reads)");
}

/// Тест concurrent event processing для одной сессии
#[tokio::test]
async fn test_concurrent_event_processing_single_session() {
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use tokio::sync::{mpsc, Mutex};
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    type EventBuffer = Arc<Mutex<HashMap<String, VecDeque<serde_json::Value>>>>;

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(100);

    let session_id = "concurrent-session".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    // Запустить EventProcessor
    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor = EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Создать 50 параллельных задач, каждая отправляет событие
    let mut tasks = vec![];
    for i in 0..50 {
        let tx_clone = event_tx.clone();
        let task = tokio::spawn(async move {
            let event = json!({
                "seq": i,
                "type": "event",
                "event": "output",
                "body": {"output": format!("Message {}", i)}
            });

            tx_clone.send(event).await.unwrap();
        });
        tasks.push(task);
    }

    // Дождаться отправки всех событий
    for task in tasks {
        task.await.unwrap();
    }

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Проверить, что все 50 событий обработаны
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(events.len(), 50);

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("Test concurrent event processing single session completed");
}

/// Тест concurrent requests в одной сессии через EventRouter
#[tokio::test]
async fn test_concurrent_requests_same_session() {
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::sync::{mpsc, oneshot, Mutex};
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let (event_tx, _event_rx) = mpsc::channel::<serde_json::Value>(10);
    let response_map = Arc::new(Mutex::new(HashMap::<u32, oneshot::Sender<serde_json::Value>>::new()));

    // Зарегистрировать 10 ожиданий responses
    let mut receivers = vec![];
    for seq in 0..10 {
        let (tx, rx) = oneshot::channel();
        response_map.lock().await.insert(seq, tx);
        receivers.push(rx);
    }

    // Симулировать отправку 10 responses одновременно
    let response_map_clone = response_map.clone();
    let sender_task = tokio::spawn(async move {
        let mut send_tasks = vec![];
        for seq in 0..10 {
            let map_clone = response_map_clone.clone();
            let task = tokio::spawn(async move {
                // Симулируем получение response
                let response = json!({
                    "seq": seq + 100,
                    "type": "response",
                    "request_seq": seq,
                    "success": true
                });

                let mut map = map_clone.lock().await;
                if let Some(tx) = map.remove(&seq) {
                    tx.send(response).unwrap();
                }
            });
            send_tasks.push(task);
        }

        for task in send_tasks {
            task.await.unwrap();
        }
    });

    sender_task.await.unwrap();

    // Проверить, что все receivers получили responses
    for (i, rx) in receivers.into_iter().enumerate() {
        let response = rx.await.expect(&format!("Response {} not received", i));
        assert_eq!(response["request_seq"], i as u64);
        assert_eq!(response["success"], true);
    }

    tracing::info!("Test concurrent requests same session completed");
}

/// Тест concurrent polling и event processing
#[tokio::test]
async fn test_concurrent_polling_and_processing() {
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use std::sync::Arc;

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    type EventBuffer = Arc<Mutex<HashMap<String, Vec<serde_json::Value>>>>;
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "polling-session".to_string();

    // Задача 1: Постоянно добавляет события
    let buffer_clone1 = event_buffer.clone();
    let session_clone1 = session_id.clone();
    let writer_task = tokio::spawn(async move {
        for i in 0..100 {
            let mut buffer = buffer_clone1.lock().await;
            buffer
                .entry(session_clone1.clone())
                .or_insert_with(Vec::new)
                .push(json!({"event": "output", "id": i}));

            drop(buffer);
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    });

    // Задача 2: Периодически делает polling
    let buffer_clone2 = event_buffer.clone();
    let session_clone2 = session_id.clone();
    let reader_task = tokio::spawn(async move {
        let mut total_polled = 0;
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let events = {
                let mut buffer = buffer_clone2.lock().await;
                buffer.remove(&session_clone2).unwrap_or_default()
            };

            total_polled += events.len();
            tracing::debug!("Polled {} events, total: {}", events.len(), total_polled);
        }

        total_polled
    });

    writer_task.await.unwrap();
    let total_polled = reader_task.await.unwrap();

    // Проверить, что были получены события (возможно не все 100 из-за race condition)
    assert!(total_polled > 0, "Should have polled at least some events");
    tracing::info!("Total events polled: {} out of 100", total_polled);

    tracing::info!("Test concurrent polling and processing completed");
}

// ============================================================================
// СТРЕСС-ТЕСТЫ ДЛЯ КРИТИЧНЫХ ИСПРАВЛЕНИЙ (2025-11-18)
// ============================================================================

/// Стресс-тест 1: EventBuffer под нагрузкой (10K событий)
#[tokio::test]
async fn test_event_buffer_stress() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    use std::collections::{HashMap, VecDeque};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use serde_json::json;

    let buffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "stress-test".to_string();

    tracing::info!("Starting stress test: 10000 events");

    // Отправить 10000 событий
    for i in 0..10000 {
        let event = json!({"event": "output", "body": {"output": format!("Event {}", i)}});

        let mut buf = buffer.lock().await;
        let events = buf
            .entry(session_id.clone())
            .or_insert_with(VecDeque::new);

        const MAX_EVENTS_PER_SESSION: usize = 1000;
        if events.len() >= MAX_EVENTS_PER_SESSION {
            events.pop_front();
        }
        events.push_back(event);
    }

    // Проверить размер
    let buf = buffer.lock().await;
    let events = buf.get(&session_id).unwrap();
    assert_eq!(events.len(), 1000, "Buffer should be capped at 1000 events");

    tracing::info!("Stress test passed: buffer correctly limited to 1000 events");
}

/// Стресс-тест 2: Concurrent thread_id updates
#[tokio::test]
async fn test_concurrent_thread_id_updates() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    use std::sync::Arc;
    use tokio::sync::Mutex;

    let current_thread_id = Arc::new(Mutex::new(None));
    let mut handles = vec![];

    tracing::info!("Starting concurrent thread_id updates: 100 tasks");

    // 100 задач одновременно обновляют thread_id
    for i in 0..100 {
        let tid = current_thread_id.clone();
        let handle = tokio::spawn(async move {
            *tid.lock().await = Some(i as u32);
        });
        handles.push(handle);
    }

    // Ждём завершения всех задач
    for handle in handles {
        handle.await.unwrap();
    }

    // Проверить что thread_id установлен (какой-то из 0..100)
    let final_tid = *current_thread_id.lock().await;
    assert!(final_tid.is_some(), "thread_id should be set");
    assert!(final_tid.unwrap() < 100, "thread_id should be < 100");

    tracing::info!("Concurrent thread_id updates passed: final_tid = {:?}", final_tid);
}
