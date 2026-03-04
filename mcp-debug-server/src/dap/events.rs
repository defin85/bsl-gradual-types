//! EventProcessor - Background task для обработки DAP событий
//!
//! Отвечает за:
//! - Получение событий от EventRouter через mpsc::Receiver
//! - Обновление shared state (current_thread_id)
//! - Буферизация событий для polling через MCP tools
//!
//! ВАЖНО: Избегание deadlock через правильную иерархию блокировок:
//! SessionManager → EventBuffer → current_thread_id

use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{mpsc, Mutex};

/// Maximum количество событий в буфере для одного session_id (overflow protection)
const MAX_EVENT_BUFFER_SIZE: usize = 1000;

/// EventBuffer - shared буфер для polling событий по session_id
/// Arc<Mutex<...>> для thread-safe access между EventProcessor и MCP tools
pub type EventBuffer = Arc<Mutex<HashMap<String, VecDeque<Value>>>>;

/// Метрики обработки событий (атомарные счетчики для lock-free access)
#[derive(Debug, Clone)]
pub struct EventCounters {
    /// Общее количество обработанных событий
    pub total_events: Arc<AtomicU64>,
    /// Количество stopped events
    pub stopped_events: Arc<AtomicU64>,
    /// Количество continued events
    pub continued_events: Arc<AtomicU64>,
    /// Количество terminated events
    pub terminated_events: Arc<AtomicU64>,
    /// Количество output events
    pub output_events: Arc<AtomicU64>,
    /// Количество неизвестных events
    pub unknown_events: Arc<AtomicU64>,
}

impl EventCounters {
    /// Создать новые счетчики с нулевыми значениями
    pub fn new() -> Self {
        Self {
            total_events: Arc::new(AtomicU64::new(0)),
            stopped_events: Arc::new(AtomicU64::new(0)),
            continued_events: Arc::new(AtomicU64::new(0)),
            terminated_events: Arc::new(AtomicU64::new(0)),
            output_events: Arc::new(AtomicU64::new(0)),
            unknown_events: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Получить snapshot всех счетчиков
    pub fn snapshot(&self) -> EventStats {
        EventStats {
            total: self.total_events.load(Ordering::Relaxed),
            stopped: self.stopped_events.load(Ordering::Relaxed),
            continued: self.continued_events.load(Ordering::Relaxed),
            terminated: self.terminated_events.load(Ordering::Relaxed),
            output: self.output_events.load(Ordering::Relaxed),
            unknown: self.unknown_events.load(Ordering::Relaxed),
        }
    }
}

impl Default for EventCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot метрик событий (для внешнего API)
#[derive(Debug, Clone)]
pub struct EventStats {
    pub total: u64,
    pub stopped: u64,
    pub continued: u64,
    pub terminated: u64,
    pub output: u64,
    pub unknown: u64,
}

/// EventProcessor - Background task для обработки DAP событий
pub struct EventProcessor {
    /// Receiver для событий от EventRouter
    event_rx: mpsc::Receiver<Value>,
    /// Shared буфер для polling событий
    event_buffer: EventBuffer,
    /// Session ID (для логирования и буферизации)
    session_id: String,
    /// Shared current_thread_id (обновляется при stopped/continued events)
    current_thread_id: Arc<Mutex<Option<u32>>>,
    /// Счетчики метрик событий
    counters: EventCounters,
}

impl EventProcessor {
    /// Создать новый EventProcessor
    pub fn new(
        event_rx: mpsc::Receiver<Value>,
        event_buffer: EventBuffer,
        session_id: String,
        current_thread_id: Arc<Mutex<Option<u32>>>,
    ) -> Self {
        Self {
            event_rx,
            event_buffer,
            session_id,
            current_thread_id,
            counters: EventCounters::new(),
        }
    }

    /// Запустить background task для обработки событий
    ///
    /// Graceful shutdown: завершается когда event_rx закрывается (while let Some)
    pub async fn run(mut self) {
        tracing::info!(session_id = %self.session_id, "EventProcessor started");

        while let Some(event) = self.event_rx.recv().await {
            self.counters.total_events.fetch_add(1, Ordering::Relaxed);

            if let Err(e) = self.process_event(event).await {
                tracing::error!(
                    session_id = %self.session_id,
                    error = %e,
                    "Failed to process event"
                );
            }
        }

        tracing::info!(
            session_id = %self.session_id,
            stats = ?self.counters.snapshot(),
            "EventProcessor stopped gracefully"
        );
    }

    /// Обработать одно DAP событие (диспетчер по типу)
    async fn process_event(&mut self, event: Value) -> anyhow::Result<()> {
        let event_type = event
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Event without 'event' field"))?;

        tracing::debug!(
            session_id = %self.session_id,
            event_type = %event_type,
            "Processing DAP event"
        );

        // Маршрутизация по типу события
        match event_type {
            "stopped" => self.handle_stopped_event(&event).await?,
            "continued" => self.handle_continued_event(&event).await?,
            "terminated" | "exited" => self.handle_terminated_event(&event).await?,
            "output" => self.handle_output_event(&event).await?,
            "initialized" => {
                // initialized event - просто добавляем в буфер (важно для launch)
                self.add_to_buffer(event).await?;
            }
            _ => {
                // Неизвестный event - логируем и добавляем в буфер
                tracing::debug!(
                    session_id = %self.session_id,
                    event_type = %event_type,
                    "Unknown event type"
                );
                self.counters.unknown_events.fetch_add(1, Ordering::Relaxed);
                self.add_to_buffer(event).await?;
            }
        }

        Ok(())
    }

    /// Обработать stopped event (обновить current_thread_id)
    async fn handle_stopped_event(&mut self, event: &Value) -> anyhow::Result<()> {
        self.counters.stopped_events.fetch_add(1, Ordering::Relaxed);

        // Извлечь threadId из body
        let thread_id = event
            .get("body")
            .and_then(|b| b.get("threadId"))
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);

        if let Some(tid) = thread_id {
            // Обновить shared current_thread_id (ВСЕГДА через .await, НЕ blocking_lock!)
            let mut guard = self.current_thread_id.lock().await;
            *guard = Some(tid);

            tracing::info!(
                session_id = %self.session_id,
                thread_id = %tid,
                "Program stopped, updated current_thread_id"
            );
        } else {
            tracing::warn!(
                session_id = %self.session_id,
                "stopped event without threadId in body"
            );
        }

        // Добавить в буфер для polling
        self.add_to_buffer(event.clone()).await?;

        Ok(())
    }

    /// Обработать continued event (обновить current_thread_id)
    async fn handle_continued_event(&mut self, event: &Value) -> anyhow::Result<()> {
        self.counters
            .continued_events
            .fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            session_id = %self.session_id,
            "Program continued execution"
        );

        // Опционально: можно сбросить current_thread_id
        // Но для удобства оставляем последний известный thread_id

        // Добавить в буфер
        self.add_to_buffer(event.clone()).await?;

        Ok(())
    }

    /// Обработать terminated/exited event
    async fn handle_terminated_event(&mut self, event: &Value) -> anyhow::Result<()> {
        self.counters
            .terminated_events
            .fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            session_id = %self.session_id,
            "Program terminated"
        );

        // Сбросить current_thread_id
        let mut guard = self.current_thread_id.lock().await;
        *guard = None;

        // Добавить в буфер
        self.add_to_buffer(event.clone()).await?;

        Ok(())
    }

    /// Обработать output event (console/stderr/stdout)
    async fn handle_output_event(&mut self, event: &Value) -> anyhow::Result<()> {
        self.counters.output_events.fetch_add(1, Ordering::Relaxed);

        // Извлечь текст из body.output
        let output_text = event
            .get("body")
            .and_then(|b| b.get("output"))
            .and_then(|o| o.as_str())
            .unwrap_or("");

        if !output_text.is_empty() {
            tracing::debug!(
                session_id = %self.session_id,
                output = %output_text.trim(),
                "Program output"
            );
        }

        // Добавить в буфер
        self.add_to_buffer(event.clone()).await?;

        Ok(())
    }

    /// Добавить событие в буфер (с overflow protection)
    async fn add_to_buffer(&self, event: Value) -> anyhow::Result<()> {
        let mut buffer = self.event_buffer.lock().await;

        let queue = buffer
            .entry(self.session_id.clone())
            .or_insert_with(VecDeque::new);

        // Overflow protection: ограничение MAX_EVENT_BUFFER_SIZE
        if queue.len() >= MAX_EVENT_BUFFER_SIZE {
            // Удалить самое старое событие (FIFO)
            queue.pop_front();
            tracing::warn!(
                session_id = %self.session_id,
                "EventBuffer overflow: dropped oldest event (max {})",
                MAX_EVENT_BUFFER_SIZE
            );
        }

        queue.push_back(event);

        Ok(())
    }

    /// Получить snapshot метрик
    pub fn stats(&self) -> EventStats {
        self.counters.snapshot()
    }
}

#[cfg(test)]
#[path = "events/tests.rs"]
mod tests;
