//! Event Router - маршрутизация DAP сообщений на event/response каналы
//!
//! Background task читает сообщения из DapReader и маршрутизирует их:
//! - Responses (type: "response") → oneshot channel для waiting request
//! - Events (type: "event") → mpsc channel для EventProcessor

use super::transport::DapReader;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Event Router - маршрутизирует DAP messages на event/response каналы
pub struct EventRouter {
    reader: DapReader,
    event_tx: mpsc::Sender<Value>,
    response_map: Arc<Mutex<HashMap<u32, oneshot::Sender<Value>>>>,
}

impl EventRouter {
    /// Создать новый EventRouter
    pub fn new(
        reader: DapReader,
        event_tx: mpsc::Sender<Value>,
        response_map: Arc<Mutex<HashMap<u32, oneshot::Sender<Value>>>>,
    ) -> Self {
        Self {
            reader,
            event_tx,
            response_map,
        }
    }

    /// Запустить background task для чтения и маршрутизации
    pub async fn run(mut self) {
        tracing::info!("EventRouter started");

        loop {
            match self.reader.receive().await {
                Ok(message) => {
                    let msg_type = message
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match msg_type {
                        "event" => {
                            let event_type = message
                                .get("event")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            tracing::debug!("Routing event: {}", event_type);

                            if let Err(e) = self.event_tx.send(message).await {
                                tracing::error!("Failed to send event to channel: {}", e);
                                break; // Channel closed
                            }
                        }
                        "response" => {
                            if let Some(seq) = message.get("request_seq").and_then(|v| v.as_u64()) {
                                let seq = seq as u32;
                                tracing::debug!("Routing response for seq: {}", seq);

                                let mut map = self.response_map.lock().await;
                                if let Some(tx) = map.remove(&seq) {
                                    if tx.send(message).is_err() {
                                        tracing::warn!(
                                            "Failed to send response: receiver dropped for seq {}",
                                            seq
                                        );
                                    }
                                } else {
                                    tracing::warn!("No waiting request for response seq: {}", seq);
                                }
                            } else {
                                tracing::warn!("Response without request_seq: {:?}", message);
                            }
                        }
                        _ => {
                            tracing::warn!("Unknown message type: {}", msg_type);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("EventRouter error: {:?}", e);
                    break; // Connection closed or error
                }
            }
        }

        tracing::info!("EventRouter stopped");
    }
}
