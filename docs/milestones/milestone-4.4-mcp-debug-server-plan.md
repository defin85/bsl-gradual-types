# Milestone 4.4: MCP Debug Server — Детальный план реализации

**Дата создания:** 2025-11-18
**Статус:** PLANNED
**Приоритет:** 🟢 ВЫСОКИЙ
**Оценка времени:** 17 дней (3.5 недели)

---

## 📋 Обзор

Создание **MCP Debug Server** с DAP bridge для AI-ассистированной интерактивной отладки программ.

**Проблема:** AI-ассистенты НЕ могут интерактивно отлаживать через debugger. Вынуждены использовать print debugging (медленно: 5-10 минут на итерацию).

**Решение:** MCP server с DAP протоколом для полноценной отладки через AI (быстро: <1 минута на итерацию).

---

## 🔍 Результаты исследования Architect

### 1.1. Существующие решения

**Найденные MCP Debug Servers:**

1. **VSCode Debugger MCP Server by Jason McGhee** (TypeScript)
   - Репозиторий: `jasonjmcghee/claude-debugs-for-you`
   - Звёзды: ~59.6k downloads
   - **Плюсы:** Работает "из коробки" в VS Code
   - **Минусы:** Привязан к VS Code environment, не standalone

2. **GDB Debugger MCP Server by Signal-Slot** (TypeScript)
   - Репозиторий: `signal-slot/mcp-gdb`
   - Звёзды: 58+ GitHub stars, ~3.2k downloads
   - **Плюсы:** Language-agnostic, standalone
   - **Минусы:** Только GDB (нет LLDB support), требует Node.js runtime

**Вывод:** Существующие решения написаны на TypeScript. **НЕТ** Rust-based MCP Debug Server с DAP support.

### 1.2. DAP (Debug Adapter Protocol) Specification

**Версия протокола:** 1.70.0 (актуальная)

**Ключевые сообщения DAP:**

| Request/Event | Назначение |
|---------------|-----------|
| `initialize` | Первый запрос для настройки capabilities |
| `launch` | Запуск программы с/без отладки |
| `setBreakpoints` | Установка breakpoints для source file |
| `continue` | Продолжение выполнения всех threads |
| `stackTrace` | Получение stack trace для thread |
| `evaluate` | Вычисление expression в контексте frame |
| `next` | Step over |
| `stepIn` | Step into |
| `stepOut` | Step out |

**Транспорт:** Content-Length headers + JSON body (аналогично LSP)

### 1.3. RMCP 0.8.5 (Официальный Rust SDK)

**Репозиторий:** `modelcontextprotocol/rust-sdk`

**Breaking changes от 0.1.5 → 0.8.5:**

- Новые macros: `#[tool]`, `#[tool_router]`, `#[prompt]`
- `Arguments<T>` → `Parameters<T>`
- `rmcp::Error` → `rmcp::ErrorData`
- Новый wrapper `Json<T>` с `IntoCallToolResult`
- Автоматическая генерация JSON schema из Rust типов

### 1.4. Rust DAP Client Libraries

**Рекомендация:** **dap-rs 0.2.0-alpha1** by sztomi
- URL: https://github.com/sztomi/dap-rs
- Полная реализация DAP client + server
- `BasicClient` с `BufWriter`/`BufReader`
- Парсинг Content-Length headers
- MIT/Apache 2.0 dual license
- **Экономит 80% времени** на реализацию протокола

---

## 🏗️ Архитектура

### Общая диаграмма

```
┌────────────────┐   JSON-RPC/MCP     ┌──────────────────────────────────────┐
│  Claude Code   │ ◄─────────────────►│  MCP Debug Server (Rust)             │
│  (AI Agent)    │   stdio/HTTP       │                                      │
└────────────────┘                    │  ┌────────────────────────────────┐  │
                                      │  │  MCP Server Layer              │  │
                                      │  │  - Tools (12 debug tools)      │  │
                                      │  │  - Resources (debug info)      │  │
                                      │  └────────────────────────────────┘  │
                                      │                │                     │
                                      │                ▼                     │
                                      │  ┌────────────────────────────────┐  │
                                      │  │  Session Manager               │  │
                                      │  │  - HashMap<SessionId, Session> │  │
                                      │  │  - Concurrent access (RwLock)  │  │
                                      │  └────────────────────────────────┘  │
                                      │                │                     │
                                      │                ▼                     │
                                      │  ┌────────────────────────────────┐  │
                                      │  │  DAP Client Wrapper            │  │
                                      │  │  - Protocol handling           │  │
                                      │  │  - Content-Length parsing      │  │
                                      │  └────────────────────────────────┘  │
                                      └──────────────────┬───────────────────┘
                                                         │ DAP Protocol
                                                         │ (stdio)
                                                         ▼
                                      ┌──────────────────────────────────────┐
                                      │  DAP Server (CodeLLDB)               │
                                      │  - Language-specific adapter         │
                                      │  - LLDB/GDB bridge                   │
                                      └──────────────────┬───────────────────┘
                                                         │
                                                         ▼
                                      ┌──────────────────────────────────────┐
                                      │  Debugger (LLDB, GDB)                │
                                      │  - Binary под отладкой               │
                                      └──────────────────────────────────────┘
```

### Структура модулей

```
mcp-debug-server/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              // Entry point: stdio MCP server
│   ├── lib.rs               // Public API
│   │
│   ├── server/              // MCP Server Layer
│   │   ├── mod.rs           // ServerHandler implementation
│   │   ├── tools.rs         // MCP Tools definitions (12 tools)
│   │   └── resources.rs     // MCP Resources (debug info)
│   │
│   ├── session/             // Session Management
│   │   ├── mod.rs           // SessionManager
│   │   ├── manager.rs       // HashMap + concurrency logic
│   │   └── state.rs         // SessionState enum + transitions
│   │
│   ├── dap/                 // DAP Client Layer
│   │   ├── mod.rs           // Public DAP API
│   │   ├── client.rs        // DapClient (wraps dap-rs)
│   │   ├── protocol.rs      // DAP message types
│   │   ├── transport.rs     // Content-Length handling
│   │   └── events.rs        // Event handling (stopped, output)
│   │
│   ├── types/               // Shared types
│   │   ├── mod.rs
│   │   ├── session_id.rs    // SessionId = String (UUID)
│   │   └── error.rs         // Custom error types
│   │
│   └── config/              // Configuration
│       ├── mod.rs
│       └── adapters.rs      // DAP adapter paths (CodeLLDB, etc.)
│
└── tests/
    ├── integration/         // Интеграционные тесты
    │   ├── basic_debug.rs   // Тест простой debug сессии
    │   └── concurrent.rs    // Тест concurrent сессий
    └── fixtures/            // Тестовые бинарники
        └── test_program.rs
```

### Зависимости (Cargo.toml)

```toml
[package]
name = "mcp-debug-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "mcp-debug"
path = "src/main.rs"

[dependencies]
rmcp = { version = "0.8.5", features = ["server", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
dap-rs = "0.2.0-alpha1"

[dev-dependencies]
tokio-test = "0.4"
```

---

## 🛠️ 12 MCP Tools

1. **`debug_create_session`** — создать новую debug сессию
   - Параметры: `binary_path: String`, `adapter_type: Option<String>`
   - Возвращает: `session_id`

2. **`debug_set_breakpoint`** — установить breakpoint
   - Параметры: `session_id`, `file`, `line`

3. **`debug_launch`** — запустить программу
   - Параметры: `session_id`, `args: Option<Vec<String>>`

4. **`debug_next`** — step over (следующая строка)
   - Параметры: `session_id`

5. **`debug_step_in`** — step into (войти в функцию)
   - Параметры: `session_id`

6. **`debug_continue`** — продолжить выполнение до breakpoint
   - Параметры: `session_id`

7. **`debug_eval`** — вычислить expression в текущем фрейме
   - Параметры: `session_id`, `expression: String`

8. **`debug_backtrace`** — показать stack trace
   - Параметры: `session_id`

9. **`debug_set_conditional_breakpoint`** — условный breakpoint
   - Параметры: `session_id`, `file`, `line`, `condition: String`

10. **`debug_list_sessions`** — список активных сессий
    - Параметры: нет

11. **`debug_terminate`** — завершить debug сессию
    - Параметры: `session_id`

12. **`debug_step_out`** (опционально) — выйти из функции
    - Параметры: `session_id`

---

## 📅 План реализации (17 дней)

| Этап | Задача | Время | Зависимости | Критерии выполнения |
|------|--------|-------|-------------|---------------------|
| **1** | Создание crate структуры + Cargo.toml | **0.5 дня** | — | Проект компилируется, зависимости установлены |
| **2** | DAP Client implementation | **3 дня** | dap-rs integration | Может запустить CodeLLDB, отправить initialize, setBreakpoints, получить response |
| **3** | Session Manager | **2 дня** | DAP Client | Может создать сессию, хранить state, concurrent access работает |
| **4** | MCP Server skeleton | **1 день** | rmcp 0.8.5 | Базовый MCP server с 1-2 dummy tools |
| **5** | MCP Tools (основные 6) | **2 дня** | Session Manager | create_session, set_breakpoint, launch, next, step_in, continue работают |
| **6** | MCP Tools (продвинутые 6) | **1.5 дня** | Этап 5 | eval, backtrace, conditional_breakpoint, list_sessions, terminate |
| **7** | MCP Resources | **1 день** | Session Manager | list_resources, read_resource возвращают JSON с session info |
| **8** | Event handling | **2 дня** | DAP Client | Обработка stopped, output, terminated events |
| **9** | Error handling + logging | **1 день** | Все компоненты | tracing/log, ErrorData mapping, graceful error recovery |
| **10** | Интеграционные тесты | **2 дня** | Все компоненты | 3-5 тестов: basic debug, concurrent sessions, breakpoints, eval |
| **11** | Документация + примеры | **1 день** | — | README.md, examples/simple_debug.rs |

**Итого:** 17 дней (3.5 недели)
**Критический путь:** 11 дней

### Граф зависимостей

```
                    ┌───────────────┐
                    │  Этап 1: Crate│
                    │  structure    │
                    └───────┬───────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
    ┌─────────────────┐         ┌─────────────────┐
    │ Этап 2: DAP     │         │ Этап 4: MCP     │
    │ Client          │         │ Server skeleton │
    └────────┬────────┘         └────────┬────────┘
             │                           │
             ▼                           │
    ┌─────────────────┐                 │
    │ Этап 3: Session │                 │
    │ Manager         │◄────────────────┘
    └────────┬────────┘
             │
             ├──────────┬──────────────┬──────────┐
             │          │              │          │
             ▼          ▼              ▼          ▼
    ┌────────────┐ ┌────────┐ ┌──────────┐ ┌─────────┐
    │ Этап 5:    │ │ Этап 7:│ │ Этап 8:  │ │ Этап 9: │
    │ MCP Tools  │ │ MCP    │ │ Event    │ │ Error   │
    │ (basic)    │ │ Res.   │ │ handling │ │ handling│
    └─────┬──────┘ └────────┘ └──────────┘ └─────────┘
          │
          ▼
    ┌────────────┐
    │ Этап 6:    │
    │ MCP Tools  │
    │ (advanced) │
    └─────┬──────┘
          │
          ▼
    ┌─────────────────┐
    │ Этап 10:        │
    │ Integration     │
    │ tests           │
    └────────┬────────┘
             │
             ▼
    ┌─────────────────┐
    │ Этап 11:        │
    │ Documentation   │
    └─────────────────┘
```

---

## ⚠️ Риски и mitigation

| Риск | Вероятность | Влияние | Mitigation |
|------|-------------|---------|------------|
| **DAP adapters (CodeLLDB, etc.) недоступны в PATH** | Средняя | Высокое | Добавить config файл для путей к adapters, документация по установке |
| **dap-rs API нестабилен (alpha версия)** | Низкая | Высокое | Использовать конкретный git commit, заморозить версию, добавить vendoring |
| **rmcp 0.8.5 breaking changes** | Низкая | Среднее | Тщательно тестировать совместимость, следить за changelog |
| **Concurrent session management deadlocks** | Средняя | Среднее | Тщательное тестирование с tokio::test, использовать tokio::sync::RwLock правильно |
| **DAP events (stopped, output) не обрабатываются вовремя** | Средняя | Высокое | Async event stream с tokio::select!, timeout handling |
| **Разные DAP adapters (LLDB vs GDB) имеют inconsistent behaviour** | Высокая | Среднее | Начать с одного adapter (CodeLLDB), добавить abstraction layer для adapter-specific quirks |

---

## 🎯 Рекомендации Architect

### Технический стек (РЕКОМЕНДУЕТСЯ)

- **Язык:** Rust (нативная интеграция с bsl-gradual-types)
- **MCP SDK:** rmcp 0.8.5 (официальный Anthropic)
- **DAP Client:** dap-rs 0.2.0-alpha1 (экономит 80% времени)
- **Транспорт:** stdio (stdin/stdout)
- **Concurrency:** tokio + `Arc<RwLock<HashMap>>`
- **Первый adapter:** CodeLLDB (лучший Rust support)

### Особенности реализации

1. **Session Management:**
   ```rust
   Arc<RwLock<HashMap<SessionId, DebugSession>>>
   ```
   ⚠️ **НЕ использовать Mutex** (риск deadlock)!

2. **Tool Output Format:**
   AI-friendly структурированный текст:
   ```
   Breakpoint set:
   - File: src/main.rs
   - Line: 42
   - Verified: true
   ```

3. **Event Handling:**
   ```rust
   async fn wait_for_stopped(&mut self) -> Result<StoppedEventData> {
       // Читаем events до получения "stopped"
   }
   ```

4. **Config для DAP Adapters:**
   ```toml
   [adapters]
   lldb = "/path/to/codelldb"
   gdb = "/path/to/gdb-dap"
   ```

### Вопросы для уточнения (рекомендации Architect)

1. **DAP Adapter приоритет:**
   - 🟢 Начать с **CodeLLDB** (LLDB для Rust)? ← РЕКОМЕНДУЕТСЯ
   - Или универсальный (все adapters)?

2. **Scope:**
   - 🟢 Только **Rust debugging**? ← РЕКОМЕНДУЕТСЯ
   - Или multi-language сразу?

3. **Deployment:**
   - 🟢 **Standalone binary** `mcp-debug`? ← РЕКОМЕНДУЕТСЯ

4. **Тестирование:**
   - 🟢 **Автоматизированные + manual guide**? ← РЕКОМЕНДУЕТСЯ

---

## 📖 Ключевые компоненты (примеры кода)

### Session Manager (src/session/manager.rs)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Initialized,
    Running,
    Stopped,
    Terminated,
}

pub struct DebugSession {
    pub id: String,
    pub dap_client: DapClient,
    pub binary_path: String,
    pub current_thread_id: Option<u32>,
    pub breakpoints: HashMap<String, Vec<u32>>,
    pub state: SessionState,
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        binary_path: String,
        adapter_type: String,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let mut dap_client = DapClient::spawn(&adapter_type).await?;
        dap_client.initialize().await?;

        let session = DebugSession {
            id: session_id.clone(),
            dap_client,
            binary_path,
            current_thread_id: None,
            breakpoints: HashMap::new(),
            state: SessionState::Initialized,
        };

        self.sessions.write().await.insert(session_id.clone(), session);
        Ok(session_id)
    }

    pub async fn with_session<F, Fut, R>(
        &self,
        session_id: &str,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&mut DebugSession) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        f(session).await
    }
}
```

### DAP Client (src/dap/client.rs)

```rust
use anyhow::Result;
use serde_json::{json, Value};
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::process::Stdio;

pub struct DapClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    seq_counter: u32,
}

impl DapClient {
    pub async fn spawn(adapter_command: &str) -> Result<Self> {
        let mut child = Command::new(adapter_command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            process: child,
            seq_counter: 1,
        })
    }

    pub async fn initialize(&mut self) -> Result<Value> {
        self.send_request("initialize", json!({
            "clientID": "mcp-debug-server",
            "adapterID": "lldb",
            "linesStartAt1": true,
            "columnsStartAt1": true,
        })).await
    }

    pub async fn set_breakpoint(&mut self, file: &str, line: u32) -> Result<Value> {
        self.send_request("setBreakpoints", json!({
            "source": { "path": file },
            "breakpoints": [{ "line": line }],
        })).await
    }

    async fn send_request(&mut self, command: &str, args: Value) -> Result<Value> {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        let request = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": args,
        });

        // DAP использует Content-Length header
        let body = serde_json::to_string(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(body.as_bytes()).await?;
        self.stdin.flush().await?;

        self.read_message().await
    }

    async fn read_message(&mut self) -> Result<Value> {
        // Читаем Content-Length header
        let mut header_line = String::new();
        self.stdout.read_line(&mut header_line).await?;

        let length: usize = header_line
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()?;

        // Пропускаем пустую строку
        self.stdout.read_line(&mut String::new()).await?;

        // Читаем JSON body
        let mut buffer = vec![0u8; length];
        self.stdout.read_exact(&mut buffer).await?;

        Ok(serde_json::from_slice(&buffer)?)
    }
}
```

### MCP Tools (src/server/tools.rs)

```rust
use rmcp::{tool, tool_router, model::CallToolResult, ErrorData};

#[tool_router]
impl DebugServer {
    #[tool(
        name = "debug_create_session",
        description = "Create a new debug session for a binary program"
    )]
    async fn create_session(
        &self,
        binary_path: String,
        adapter_type: Option<String>,
    ) -> Result<CallToolResult, ErrorData> {
        let adapter = adapter_type.unwrap_or_else(|| "lldb".to_string());

        let session_id = self.session_manager
            .create_session(binary_path.clone(), adapter)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string()))?;

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Debug session created:\n\
                 - Session ID: {}\n\
                 - Binary: {}\n\
                 - Adapter: {}",
                session_id, binary_path, adapter
            ))
        ]))
    }

    #[tool(
        name = "debug_set_breakpoint",
        description = "Set a breakpoint at file:line"
    )]
    async fn set_breakpoint(
        &self,
        session_id: String,
        file: String,
        line: u32,
    ) -> Result<CallToolResult, ErrorData> {
        // Implementation...
    }

    // ... другие 10 tools
}
```

---

## 🚀 Следующие шаги

1. ✅ **План создан** (этот документ)
2. ⏳ **Создать git worktree** для разработки
3. ⏳ **Начать Этап 1** — создать crate структуру
4. ⏳ **Этап 2** — интегрировать dap-rs
5. ⏳ **Этап 3-11** — последовательная реализация компонентов

---

## 📚 Ссылки

- **DAP Specification:** https://microsoft.github.io/debug-adapter-protocol/
- **RMCP SDK:** https://github.com/modelcontextprotocol/rust-sdk
- **dap-rs:** https://github.com/sztomi/dap-rs
- **CodeLLDB:** https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb
- **ROADMAP_2025.md:** [Milestone 4.4, строки 2706-3512](../../ROADMAP_2025.md)

---

**Версия документа:** 1.0
**Автор:** Architect Agent + Claude Code
**Последнее обновление:** 2025-11-18
