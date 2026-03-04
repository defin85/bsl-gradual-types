use super::*;

#[test]
fn test_event_counters() {
    let counters = EventCounters::new();

    counters.total_events.fetch_add(1, Ordering::Relaxed);
    counters.stopped_events.fetch_add(1, Ordering::Relaxed);

    let stats = counters.snapshot();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.stopped, 1);
    assert_eq!(stats.continued, 0);
}

#[tokio::test]
async fn test_event_buffer_overflow_protection() {
    let buffer = Arc::new(Mutex::new(HashMap::new()));
    let (_tx, rx) = mpsc::channel(100);
    let current_thread_id = Arc::new(Mutex::new(None));

    let processor = EventProcessor::new(
        rx,
        buffer.clone(),
        "test-session".to_string(),
        current_thread_id,
    );

    // Добавить MAX_EVENT_BUFFER_SIZE + 1 событий
    for i in 0..=MAX_EVENT_BUFFER_SIZE {
        let event = serde_json::json!({
            "type": "event",
            "event": "output",
            "body": { "output": format!("Message {}", i) }
        });
        processor.add_to_buffer(event).await.unwrap();
    }

    // Проверить, что в буфере ровно MAX_EVENT_BUFFER_SIZE
    let buf = buffer.lock().await;
    let queue = buf.get("test-session").unwrap();
    assert_eq!(queue.len(), MAX_EVENT_BUFFER_SIZE);

    // Проверить, что первое событие было удалено (FIFO)
    let first_event = queue.front().unwrap();
    let output = first_event["body"]["output"].as_str().unwrap();
    assert_eq!(output, "Message 1"); // Message 0 был удален
}
