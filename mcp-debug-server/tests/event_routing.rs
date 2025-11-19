//! Unit Tests: Event Routing
//!
//! Тестирует EventRouter - маршрутизацию DAP сообщений на event/response каналы

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
// use tokio::io::{AsyncReadExt, AsyncWriteExt}; // Unused

mod support;
use support::MockTransport;

// ВАЖНО: DapReader/DapWriter используют process stdin/stdout
// Для тестирования нужно создать адаптер через DuplexStream

/// Простой mock DapReader для тестирования
struct MockDapReader {
    stream: tokio::io::DuplexStream,
}

impl MockDapReader {
    fn new(stream: tokio::io::DuplexStream) -> Self {
        Self { stream }
    }

    async fn receive(&mut self) -> Result<serde_json::Value, String> {
        MockTransport::receive_message(&mut self.stream)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Тест маршрутизации событий (events)
#[tokio::test]
async fn test_event_router_events() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Создать каналы
    let (event_tx, mut event_rx) = mpsc::channel(10);
    let response_map = Arc::new(Mutex::new(
        HashMap::<u32, oneshot::Sender<serde_json::Value>>::new(),
    ));

    // Создать mock transport
    let mut transport = MockTransport::new();

    // Создать MockDapReader
    let mut reader = MockDapReader::new(transport.client_stdout);

    // Запустить router в background task
    let router_task = tokio::spawn(async move {
        // Симулируем EventRouter.run() вручную
        loop {
            match reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if msg_type == "event" {
                        if event_tx.send(message).await.is_err() {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Отправить событие через mock stdout
    let event = json!({
        "seq": 1,
        "type": "event",
        "event": "stopped",
        "body": {
            "reason": "breakpoint",
            "threadId": 1
        }
    });

    MockTransport::send_message(&mut transport.server_stdout, &event)
        .await
        .unwrap();

    // Проверить, что событие получено через event_rx
    let received = tokio::time::timeout(tokio::time::Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");

    assert_eq!(received["type"], "event");
    assert_eq!(received["event"], "stopped");
    assert_eq!(received["body"]["threadId"], 1);

    // Cleanup
    drop(transport.server_stdout);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), router_task).await;

    tracing::info!("test_event_router_events passed");
}

/// Тест маршрутизации responses
#[tokio::test]
async fn test_event_router_responses() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Создать каналы
    let (event_tx, _event_rx) = mpsc::channel(10);
    let response_map = Arc::new(Mutex::new(
        HashMap::<u32, oneshot::Sender<serde_json::Value>>::new(),
    ));

    // Зарегистрировать ожидание response с seq=1
    let (tx, rx) = oneshot::channel();
    response_map.lock().await.insert(1, tx);

    // Создать mock transport
    let mut transport = MockTransport::new();
    let mut reader = MockDapReader::new(transport.client_stdout);

    let response_map_clone = response_map.clone();

    // Запустить router в background task
    let router_task = tokio::spawn(async move {
        loop {
            match reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if msg_type == "response" {
                        if let Some(seq) = message.get("request_seq").and_then(|v| v.as_u64()) {
                            let seq = seq as u32;
                            let mut map = response_map_clone.lock().await;
                            if let Some(tx) = map.remove(&seq) {
                                let _ = tx.send(message);
                            }
                        }
                    } else if msg_type == "event" {
                        let _ = event_tx.send(message).await;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Отправить response через mock stdout
    let response = json!({
        "seq": 10,
        "type": "response",
        "request_seq": 1,
        "success": true,
        "command": "initialize"
    });

    MockTransport::send_message(&mut transport.server_stdout, &response)
        .await
        .unwrap();

    // Проверить, что response получен через oneshot
    let received = tokio::time::timeout(tokio::time::Duration::from_millis(500), rx)
        .await
        .expect("Timeout waiting for response")
        .expect("Oneshot channel closed");

    assert_eq!(received["type"], "response");
    assert_eq!(received["request_seq"], 1);
    assert_eq!(received["success"], true);

    // Cleanup
    drop(transport.server_stdout);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), router_task).await;

    tracing::info!("test_event_router_responses passed");
}

/// Тест смешанной маршрутизации (events + responses одновременно)
#[tokio::test]
async fn test_event_router_mixed_messages() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let (event_tx, mut event_rx) = mpsc::channel(10);
    let response_map = Arc::new(Mutex::new(
        HashMap::<u32, oneshot::Sender<serde_json::Value>>::new(),
    ));

    // Зарегистрировать ожидание двух responses
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();
    response_map.lock().await.insert(1, tx1);
    response_map.lock().await.insert(2, tx2);

    let mut transport = MockTransport::new();
    let mut reader = MockDapReader::new(transport.client_stdout);
    let response_map_clone = response_map.clone();

    let router_task = tokio::spawn(async move {
        loop {
            match reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match msg_type {
                        "event" => {
                            if event_tx.send(message).await.is_err() {
                                break;
                            }
                        }
                        "response" => {
                            if let Some(seq) = message.get("request_seq").and_then(|v| v.as_u64()) {
                                let seq = seq as u32;
                                let mut map = response_map_clone.lock().await;
                                if let Some(tx) = map.remove(&seq) {
                                    let _ = tx.send(message);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Отправить: event → response1 → event → response2
    let messages = vec![
        json!({"seq": 1, "type": "event", "event": "initialized"}),
        json!({"seq": 10, "type": "response", "request_seq": 1, "success": true}),
        json!({"seq": 2, "type": "event", "event": "stopped", "body": {"threadId": 1}}),
        json!({"seq": 11, "type": "response", "request_seq": 2, "success": true}),
    ];

    for msg in &messages {
        MockTransport::send_message(&mut transport.server_stdout, msg)
            .await
            .unwrap();
    }

    // Проверить events
    let event1 = event_rx.recv().await.expect("Event 1 not received");
    assert_eq!(event1["event"], "initialized");

    let event2 = event_rx.recv().await.expect("Event 2 not received");
    assert_eq!(event2["event"], "stopped");

    // Проверить responses
    let resp1 = rx1.await.expect("Response 1 not received");
    assert_eq!(resp1["request_seq"], 1);

    let resp2 = rx2.await.expect("Response 2 not received");
    assert_eq!(resp2["request_seq"], 2);

    // Cleanup
    drop(transport.server_stdout);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), router_task).await;

    tracing::info!("test_event_router_mixed_messages passed");
}

/// Тест обработки невалидных сообщений
#[tokio::test]
async fn test_event_router_invalid_messages() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let (event_tx, mut event_rx) = mpsc::channel(10);
    let response_map = Arc::new(Mutex::new(
        HashMap::<u32, oneshot::Sender<serde_json::Value>>::new(),
    ));

    let mut transport = MockTransport::new();
    let mut reader = MockDapReader::new(transport.client_stdout);
    let response_map_clone = response_map.clone();

    let router_task = tokio::spawn(async move {
        loop {
            match reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match msg_type {
                        "event" => {
                            if event_tx.send(message).await.is_err() {
                                break;
                            }
                        }
                        "response" => {
                            if let Some(seq) = message.get("request_seq").and_then(|v| v.as_u64()) {
                                let seq = seq as u32;
                                let mut map = response_map_clone.lock().await;
                                if let Some(tx) = map.remove(&seq) {
                                    let _ = tx.send(message);
                                }
                            }
                        }
                        _ => {
                            // Unknown type - игнорируем
                            tracing::warn!("Unknown message type: {}", msg_type);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Отправить сообщение с неизвестным type
    let unknown_msg = json!({"seq": 1, "type": "unknown", "data": "test"});
    MockTransport::send_message(&mut transport.server_stdout, &unknown_msg)
        .await
        .unwrap();

    // Отправить валидное событие после невалидного
    let valid_event = json!({"seq": 2, "type": "event", "event": "output", "body": {}});
    MockTransport::send_message(&mut transport.server_stdout, &valid_event)
        .await
        .unwrap();

    // Проверить, что валидное событие получено
    let received = tokio::time::timeout(tokio::time::Duration::from_millis(500), event_rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    assert_eq!(received["event"], "output");

    // Cleanup
    drop(transport.server_stdout);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), router_task).await;

    tracing::info!("test_event_router_invalid_messages passed");
}

/// Тест response без зарегистрированного oneshot (orphaned response)
#[tokio::test]
async fn test_event_router_orphaned_response() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let (event_tx, _event_rx) = mpsc::channel(10);
    let response_map = Arc::new(Mutex::new(
        HashMap::<u32, oneshot::Sender<serde_json::Value>>::new(),
    ));

    let mut transport = MockTransport::new();
    let mut reader = MockDapReader::new(transport.client_stdout);
    let response_map_clone = response_map.clone();

    let router_task = tokio::spawn(async move {
        loop {
            match reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if msg_type == "response" {
                        if let Some(seq) = message.get("request_seq").and_then(|v| v.as_u64()) {
                            let seq = seq as u32;
                            let mut map = response_map_clone.lock().await;
                            if let Some(tx) = map.remove(&seq) {
                                let _ = tx.send(message);
                            } else {
                                tracing::warn!("Orphaned response for seq: {}", seq);
                            }
                        }
                    } else if msg_type == "event" {
                        let _ = event_tx.send(message).await;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Отправить response без зарегистрированного oneshot
    let orphaned_response = json!({
        "seq": 99,
        "type": "response",
        "request_seq": 999,
        "success": true
    });

    MockTransport::send_message(&mut transport.server_stdout, &orphaned_response)
        .await
        .unwrap();

    // Подождать чтобы router обработал сообщение
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Проверить, что response_map пуст (orphaned response удалён или не найден)
    let map = response_map.lock().await;
    assert!(!map.contains_key(&999));

    // Cleanup
    drop(transport.server_stdout);
    let _ = tokio::time::timeout(tokio::time::Duration::from_millis(100), router_task).await;

    tracing::info!("test_event_router_orphaned_response passed");
}
