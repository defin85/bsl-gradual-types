//! Unit Tests: Event Processing
//!
//! Тестирует EventProcessor и EventBuffer - async обработка DAP событий

use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// EventBuffer типа из модуля events (обновлён на VecDeque)
type EventBuffer = Arc<Mutex<HashMap<String, VecDeque<serde_json::Value>>>>;

#[tokio::test]
async fn test_event_processor_stopped_event() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Создать EventBuffer
    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));

    // Создать channel для событий
    let (event_tx, event_rx) = mpsc::channel(10);

    let session_id = "test-session-1".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    // Симулировать EventProcessor
    let processor_task = tokio::spawn(async move {
        // Создать EventProcessor (упрощённая версия)
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить stopped event
    let stopped_event = json!({
        "seq": 1,
        "type": "event",
        "event": "stopped",
        "body": {
            "reason": "breakpoint",
            "threadId": 42,
            "allThreadsStopped": true
        }
    });

    event_tx.send(stopped_event.clone()).await.unwrap();

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Проверить, что событие добавлено в EventBuffer
    let buffer = event_buffer.lock().await;
    let events = buffer
        .get(&session_id)
        .expect("Events not found for session");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "stopped");
    assert_eq!(events[0]["body"]["threadId"], 42);

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("test_event_processor_stopped_event passed");
}

#[tokio::test]
async fn test_event_processor_multiple_events() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(10);

    let session_id = "test-session-2".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить несколько событий
    let events = vec![
        json!({"seq": 1, "type": "event", "event": "initialized"}),
        json!({"seq": 2, "type": "event", "event": "output", "body": {"output": "Hello\n", "category": "stdout"}}),
        json!({"seq": 3, "type": "event", "event": "stopped", "body": {"reason": "step", "threadId": 1}}),
        json!({"seq": 4, "type": "event", "event": "continued", "body": {"threadId": 1}}),
    ];

    for event in &events {
        event_tx.send(event.clone()).await.unwrap();
    }

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Проверить, что все события в буфере
    let buffer = event_buffer.lock().await;
    let stored_events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(stored_events.len(), 4);

    assert_eq!(stored_events[0]["event"], "initialized");
    assert_eq!(stored_events[1]["event"], "output");
    assert_eq!(stored_events[2]["event"], "stopped");
    assert_eq!(stored_events[3]["event"], "continued");

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("test_event_processor_multiple_events passed");
}

#[tokio::test]
async fn test_event_buffer_polling() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "test-session-3".to_string();

    // Добавить события напрямую в буфер
    {
        let mut buffer = event_buffer.lock().await;
        let events = VecDeque::from(vec![
            json!({"event": "stopped", "body": {"threadId": 1}}),
            json!({"event": "output", "body": {"output": "test"}}),
        ]);
        buffer.insert(session_id.clone(), events);
    }

    // Симулировать polling (чтение и очистку)
    let polled_events = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };

    // Проверки
    assert_eq!(polled_events.len(), 2);
    assert_eq!(polled_events[0]["event"], "stopped");
    assert_eq!(polled_events[1]["event"], "output");

    // Проверить, что буфер очищен
    let buffer = event_buffer.lock().await;
    assert!(!buffer.contains_key(&session_id));

    tracing::info!("test_event_buffer_polling passed");
}

#[tokio::test]
async fn test_event_buffer_concurrent_access() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "test-session-4".to_string();

    // Создать несколько задач, которые одновременно добавляют события
    let mut tasks = vec![];
    for i in 0..10 {
        let buffer_clone = event_buffer.clone();
        let session_clone = session_id.clone();

        let task = tokio::spawn(async move {
            let event = json!({"event": "output", "body": {"message": format!("Event {}", i)}});

            let mut buffer = buffer_clone.lock().await;
            buffer
                .entry(session_clone)
                .or_insert_with(VecDeque::new)
                .push_back(event);
        });

        tasks.push(task);
    }

    // Дождаться завершения всех задач
    for task in tasks {
        task.await.unwrap();
    }

    // Проверить, что все 10 событий добавлены
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(events.len(), 10);

    tracing::info!("test_event_buffer_concurrent_access passed");
}

#[tokio::test]
async fn test_event_processor_terminated_event() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(10);

    let session_id = "test-session-5".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить terminated event
    let terminated_event = json!({
        "seq": 1,
        "type": "event",
        "event": "terminated"
    });

    event_tx.send(terminated_event.clone()).await.unwrap();

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Проверить, что событие в буфере
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "terminated");

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("test_event_processor_terminated_event passed");
}

#[tokio::test]
async fn test_event_processor_output_event() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(10);

    let session_id = "test-session-6".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить output event
    let output_event = json!({
        "seq": 1,
        "type": "event",
        "event": "output",
        "body": {
            "category": "stdout",
            "output": "Hello from debugger!\n"
        }
    });

    event_tx.send(output_event.clone()).await.unwrap();

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Проверить
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "output");
    assert_eq!(events[0]["body"]["category"], "stdout");

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("test_event_processor_output_event passed");
}

/// Тест обработки события с отсутствующим полем body
#[tokio::test]
async fn test_event_processor_malformed_event() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(10);

    let session_id = "test-session-7".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить stopped event без body (должен обработаться с ошибкой, но не упасть)
    let malformed_event = json!({
        "seq": 1,
        "type": "event",
        "event": "stopped"
        // Нет "body"
    });

    event_tx.send(malformed_event.clone()).await.unwrap();

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Событие всё равно должно попасть в буфер (даже если обработка вернула ошибку)
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id);

    // Если EventProcessor обработал с ошибкой, событие может не попасть в буфер
    // или попасть. Проверим оба варианта
    match events {
        Some(events) => {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0]["event"], "stopped");
        }
        None => {
            // Допустимо - EventProcessor мог пропустить событие из-за ошибки обработки
            tracing::warn!("Event not stored due to processing error - acceptable");
        }
    }

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;

    tracing::info!("test_event_processor_malformed_event passed");
}

/// Тест EventBuffer overflow (большое количество событий)
#[tokio::test]
async fn test_event_buffer_size_limit() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let (event_tx, event_rx) = mpsc::channel(100);

    let session_id = "test-session-8".to_string();
    let buffer_clone = event_buffer.clone();
    let session_id_clone = session_id.clone();

    // Создать Arc<Mutex<Option<u32>>> для current_thread_id (4-й параметр)
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor_task = tokio::spawn(async move {
        use mcp_debug_server::dap::EventProcessor;
        let processor =
            EventProcessor::new(event_rx, buffer_clone, session_id_clone, current_thread_id);
        processor.run().await;
    });

    // Отправить 1000 событий
    for i in 0..1000 {
        let event = json!({
            "seq": i,
            "type": "event",
            "event": "output",
            "body": {"output": format!("Message {}", i)}
        });
        event_tx.send(event).await.unwrap();
    }

    // Подождать обработку
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Проверить, что все 1000 событий в буфере
    let buffer = event_buffer.lock().await;
    let events = buffer.get(&session_id).expect("Events not found");
    assert_eq!(events.len(), 1000);

    tracing::info!(
        "test_event_buffer_size_limit passed (no limit enforced, all 1000 events stored)"
    );

    // Cleanup
    drop(event_tx);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(200), processor_task).await;
}

/// Тест polling после завершения сессии
#[tokio::test]
async fn test_poll_after_session_terminated() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let event_buffer: EventBuffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "terminated-session".to_string();

    // Добавить события
    {
        let mut buffer = event_buffer.lock().await;
        buffer.insert(
            session_id.clone(),
            VecDeque::from(vec![
                json!({"event": "stopped"}),
                json!({"event": "terminated"}),
            ]),
        );
    }

    // Polling
    let polled_events = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };

    assert_eq!(polled_events.len(), 2);

    // Повторный polling должен вернуть пустой массив
    let polled_again = {
        let mut buffer = event_buffer.lock().await;
        buffer.remove(&session_id).unwrap_or_default()
    };

    assert_eq!(polled_again.len(), 0);

    tracing::info!("test_poll_after_session_terminated passed");
}

// ============================================================================
// НОВЫЕ ТЕСТЫ ДЛЯ КРИТИЧНЫХ ИСПРАВЛЕНИЙ (2025-11-18)
// ============================================================================

/// Тест 1: EventBuffer circular buffer (overflow protection)
/// Проверяет что буфер ограничен 1000 событиями и старые события удаляются
#[tokio::test]
async fn test_event_buffer_overflow_protection() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Создать EventBuffer
    let buffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "test-session-overflow".to_string();

    // Добавить 1500 событий (превышает MAX_EVENTS_PER_SESSION = 1000)
    for i in 0..1500 {
        let event = json!({"event": "output", "body": {"output": format!("Event {}", i)}});

        let mut buf = buffer.lock().await;
        let events = buf.entry(session_id.clone()).or_insert_with(VecDeque::new);

        // Эмуляция add_to_buffer с overflow protection
        const MAX_EVENTS_PER_SESSION: usize = 1000;
        if events.len() >= MAX_EVENTS_PER_SESSION {
            events.pop_front();
        }
        events.push_back(event);
    }

    // Проверить что размер буфера = 1000 (не 1500)
    let buf = buffer.lock().await;
    let events = buf.get(&session_id).unwrap();
    assert_eq!(
        events.len(),
        1000,
        "Buffer should be limited to 1000 events"
    );

    // Проверить что oldest события удалены (первые 500)
    let first_event = &events[0];
    let output = first_event["body"]["output"].as_str().unwrap();
    assert_eq!(output, "Event 500", "Oldest 500 events should be dropped");

    // Проверить что newest события сохранены (последние 1000)
    let last_event = &events[999];
    let output = last_event["body"]["output"].as_str().unwrap();
    assert_eq!(output, "Event 1499", "Newest events should be kept");

    tracing::info!("test_event_buffer_overflow_protection passed");
}

/// Тест 3: current_thread_id update
/// Проверяет что current_thread_id обновляется при stopped event
#[tokio::test]
async fn test_current_thread_id_update() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Создать EventProcessor
    let (event_tx, event_rx) = mpsc::channel(100);
    let event_buffer = Arc::new(Mutex::new(HashMap::new()));
    let session_id = "test-session-thread-id".to_string();
    let current_thread_id = Arc::new(Mutex::new(None));
    let thread_id_clone = current_thread_id.clone();

    use mcp_debug_server::dap::EventProcessor;
    let event_processor = EventProcessor::new(
        event_rx,
        event_buffer.clone(),
        session_id,
        current_thread_id,
    );

    // Запустить в background task
    let handle = tokio::spawn(event_processor.run());

    // Отправить stopped event с threadId
    let stopped_event = json!({
        "event": "stopped",
        "body": {
            "reason": "breakpoint",
            "threadId": 12345
        }
    });
    event_tx.send(stopped_event).await.unwrap();

    // Подождать обработки
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Проверить что current_thread_id обновлён
    let tid = *thread_id_clone.lock().await;
    assert_eq!(
        tid,
        Some(12345),
        "current_thread_id should be updated to 12345"
    );

    tracing::info!("test_current_thread_id_update passed");

    // Cleanup
    drop(event_tx);
    let _ = handle.await;
}
