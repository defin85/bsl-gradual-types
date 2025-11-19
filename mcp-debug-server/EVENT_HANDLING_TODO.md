# Event Handling — TODO для будущей реализации

## Текущее состояние

**Этап 8 (упрощённая версия):** Создана заглушка EventHandler с документацией.

**Проблема:** AI НЕ получает real-time уведомления о событиях отладки.

## Что нужно для полной реализации

### 1. Background Task для чтения DAP events

```rust
// В DapClient::spawn()
let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);

tokio::spawn(async move {
    loop {
        match transport.receive().await {
            Ok(message) if message["type"] == "event" => {
                event_tx.send(message).await.ok();
            }
            _ => {} // Response или ошибка
        }
    }
});
```

### 2. Integration с MCP Server

rmcp 0.9.0 пока **НЕ поддерживает** server-initiated notifications из Rust кода.

**Workaround:**
- Использовать custom transport layer
- Или polling через MCP tool `debug_poll_events`

### 3. Event Loop с tokio::select!

```rust
loop {
    tokio::select! {
        Some(event) = event_rx.recv() => {
            // Обработать DAP event
            event_handler.handle_event(event).await;
        }
        Some(request) = mcp_rx.recv() => {
            // Обработать MCP request
            handle_mcp_request(request).await;
        }
    }
}
```

## Альтернативное решение (polling)

Реализовать MCP tool `debug_poll_events`:

```rust
#[tool(description = "Poll for debug events (stopped, output, terminated)")]
async fn debug_poll_events(&self, session_id: String) -> String {
    // Проверить pending events для session
    // Вернуть список событий с последнего poll
}
```

AI периодически вызывает этот tool (каждые 100-500ms).

**Плюсы:** Простая реализация
**Минусы:** Задержка 100-500ms

## Пример полной реализации EventHandler

### Структура с notification channel

```rust
use tokio::sync::mpsc;
use serde_json::Value;

pub struct EventHandler {
    // Channel для отправки MCP notifications
    notification_tx: mpsc::Sender<McpNotification>,
}

#[derive(Debug, Clone)]
pub struct McpNotification {
    pub method: String,
    pub params: Value,
}

impl EventHandler {
    pub fn new(notification_tx: mpsc::Sender<McpNotification>) -> Self {
        Self { notification_tx }
    }

    pub async fn handle_event(&self, event: Value) -> Result<()> {
        let event_type = event
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing event type"))?;

        match event_type {
            "stopped" => self.handle_stopped_event(event).await?,
            "continued" => self.handle_continued_event(event).await?,
            "terminated" => self.handle_terminated_event(event).await?,
            "output" => self.handle_output_event(event).await?,
            "breakpoint" => self.handle_breakpoint_event(event).await?,
            _ => {
                tracing::warn!("Unknown event type: {}", event_type);
            }
        }

        Ok(())
    }

    async fn handle_stopped_event(&self, event: Value) -> Result<()> {
        let body = event
            .get("body")
            .ok_or_else(|| anyhow!("Missing body"))?;

        let reason = body
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let thread_id = body
            .get("threadId")
            .and_then(|v| v.as_u64());

        let notification = McpNotification {
            method: "debug/stopped".to_string(),
            params: json!({
                "reason": reason,
                "threadId": thread_id,
            }),
        };

        self.notification_tx.send(notification).await?;
        Ok(())
    }

    async fn handle_output_event(&self, event: Value) -> Result<()> {
        let body = event
            .get("body")
            .ok_or_else(|| anyhow!("Missing body"))?;

        let category = body
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("console");

        let output = body
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let notification = McpNotification {
            method: "debug/output".to_string(),
            params: json!({
                "category": category,
                "output": output,
            }),
        };

        self.notification_tx.send(notification).await?;
        Ok(())
    }

    async fn handle_terminated_event(&self, _event: Value) -> Result<()> {
        let notification = McpNotification {
            method: "debug/terminated".to_string(),
            params: json!({}),
        };

        self.notification_tx.send(notification).await?;
        Ok(())
    }

    // Аналогично для continued, breakpoint...
}
```

### Integration в MCP Server

```rust
use rmcp::{Server, ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Создать channel для notifications
    let (notification_tx, mut notification_rx) = mpsc::channel(100);

    // Создать EventHandler
    let event_handler = EventHandler::new(notification_tx);

    // Создать DapClient
    let dap_client = DapClient::spawn().await?;

    // Background task для чтения DAP events
    let (event_tx, mut event_rx) = mpsc::channel(100);
    tokio::spawn(async move {
        loop {
            match dap_client.receive_event().await {
                Ok(event) => {
                    event_tx.send(event).await.ok();
                }
                Err(e) => {
                    tracing::error!("Error receiving DAP event: {:?}", e);
                    break;
                }
            }
        }
    });

    // Main event loop
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    if let Err(e) = event_handler.handle_event(event).await {
                        tracing::error!("Error handling event: {:?}", e);
                    }
                }
                Some(notification) = notification_rx.recv() => {
                    // TODO: Отправить notification в MCP Server
                    // (rmcp 0.9.0 не поддерживает server-initiated notifications)
                    tracing::info!("MCP Notification: {:?}", notification);
                }
            }
        }
    });

    // Запустить MCP Server
    let server = Server::new(ServerConfig::default());
    server.run().await?;

    Ok(())
}
```

## Ограничения rmcp 0.9.0

**Проблема:** rmcp SDK (по состоянию на 2025-11-18) НЕ предоставляет API для отправки server-initiated notifications.

**Решение для будущих версий:**
1. Ждать обновления rmcp SDK с поддержкой notifications
2. Использовать custom transport layer поверх rmcp
3. Реализовать polling через MCP tool (см. выше)

## Рекомендации

Для **Milestone 4.4** достаточно **заглушки** (текущая реализация).

Полная реализация может быть добавлена в:
- **Milestone 4.5** — если rmcp SDK обновится
- **Milestone 5.0** — с custom transport layer
- Или через **polling approach** (простое решение)

## Альтернатива: Polling Events Tool

Самое простое решение для текущего этапа — добавить MCP tool:

```rust
#[derive(Debug, Clone)]
pub struct DebugServerTools {
    session_manager: Arc<Mutex<SessionManager>>,
    event_buffer: Arc<Mutex<HashMap<String, Vec<Value>>>>, // session_id -> events
}

#[tool(description = "Poll for debug events since last call")]
async fn debug_poll_events(
    &self,
    session_id: String,
) -> Result<String, String> {
    let mut buffer = self.event_buffer.lock().await;
    let events = buffer
        .entry(session_id.clone())
        .or_insert_with(Vec::new);

    if events.is_empty() {
        return Ok("No new events".to_string());
    }

    // Вернуть события и очистить buffer
    let result = serde_json::to_string_pretty(&events)
        .map_err(|e| e.to_string())?;
    events.clear();

    Ok(result)
}
```

AI может периодически вызывать этот tool:

```javascript
// В Claude Code
while (debugging) {
    const events = await mcpClient.callTool("debug_poll_events", {
        session_id: currentSessionId
    });

    if (events !== "No new events") {
        console.log("New debug events:", events);
        // Обработать события
    }

    await sleep(200); // Poll каждые 200ms
}
```

**Плюсы:**
- Простая реализация (нет background tasks)
- Работает с текущим rmcp SDK
- AI получает события с приемлемой задержкой (200-500ms)

**Минусы:**
- Не real-time (задержка 200-500ms)
- AI должен активно polling (расход ресурсов)

---

**Вывод:** Для Milestone 4.4 рекомендуется **polling approach** как самое простое и работающее решение.
