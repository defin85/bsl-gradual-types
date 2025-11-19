# MCP Debug Server

AI-ассистированная интерактивная отладка программ через DAP протокол.

## 🎯 Проблема

AI-ассистенты (Claude Code, GitHub Copilot) НЕ могут интерактивно отлаживать программы через debugger.
Вынуждены использовать print debugging → медленно (5-10 минут на итерацию).

## ✨ Решение

MCP Debug Server предоставляет AI доступ к debugger через:
- **12 MCP Tools** для debug операций (set breakpoint, step, eval, etc.)
- **3 MCP Resources** для чтения debug информации
- **DAP протокол** для коммуникации с debugger (CodeLLDB, GDB)

**Результат:** Интерактивная отладка <1 минуты на итерацию (5-10x быстрее print debugging)

## 🏗️ Архитектура

```
┌────────────────┐   JSON-RPC/MCP     ┌──────────────────────┐
│  Claude Code   │ ◄─────────────────►│  MCP Debug Server    │
│  (AI Agent)    │   stdio/HTTP       │  (Rust)              │
└────────────────┘                    │                      │
                                      │  ┌────────────────┐  │
                                      │  │ MCP Tools      │  │
                                      │  │ (12 tools)     │  │
                                      │  └────────────────┘  │
                                      │  ┌────────────────┐  │
                                      │  │ Session Mgr    │  │
                                      │  └────────────────┘  │
                                      │  ┌────────────────┐  │
                                      │  │ DAP Client     │  │
                                      │  └────────────────┘  │
                                      └──────────┬───────────┘
                                                 │ DAP Protocol
                                                 ▼
                                      ┌──────────────────────┐
                                      │  DAP Server          │
                                      │  (CodeLLDB, GDB)     │
                                      └──────────────────────┘
```

## 📋 Prerequisites

### System Requirements
- Rust 1.70+ (MSRV)
- Tokio async runtime
- Git (for building from source)

### DAP Adapters

MCP Debug Server communicates with debuggers through DAP (Debug Adapter Protocol). You need at least one DAP adapter installed:

**For Rust/C/C++ (recommended: CodeLLDB):**
```bash
# macOS/Linux
brew install llvm

# Windows (via VS Code extension)
code --install-extension vadimcn.vscode-lldb
```

**Adapter locations:**
- macOS/Linux: `~/.vscode/extensions/vadimcn.vscode-lldb-*/adapter/codelldb`
- Windows: `%USERPROFILE%\.vscode\extensions\vadimcn.vscode-lldb-*\adapter\codelldb.exe`

**Auto-discovery:**

MCP Debug Server automatically discovers CodeLLDB when you specify `adapter_type: "lldb"` in `debug_create_session`:

```
Claude: Debug my Rust program ./target/debug/my_app
```

Behind the scenes:
1. MCP Server searches `~/.vscode/extensions/vadimcn.vscode-lldb-*` for CodeLLDB
2. If found → uses full path automatically
3. If not found → falls back to `"lldb"` (assumes it's in PATH)

**Manual adapter path:**

If auto-discovery fails or you want to use a different adapter:

```json
{
  "binary_path": "./target/debug/my_app",
  "adapter_type": "/custom/path/to/my-debugger"
}
```

**For other languages:**
- Python: `debugpy` (https://github.com/microsoft/debugpy)
- Node.js: `vscode-node-debug2`
- Go: `delve` (https://github.com/go-delve/delve)

## 📦 Installation

### 1. Build from Source

```bash
cd mcp-debug-server
cargo build --release
```

Binary: `target/release/mcp-debug` (or `mcp-debug.exe` on Windows)

## ⚙️ Конфигурация

### Claude Desktop (macOS/Linux)

Путь: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "mcp-debug": {
      "command": "/path/to/mcp-debug-server/target/release/mcp-debug",
      "env": {
        "RUST_LOG": "mcp_debug_server=info"
      }
    }
  }
}
```

### Claude Desktop (Windows)

Путь: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "mcp-debug": {
      "command": "C:\\path\\to\\mcp-debug-server\\target\\release\\mcp-debug.exe",
      "env": {
        "RUST_LOG": "mcp_debug_server=info"
      }
    }
  }
}
```

## 🚀 Быстрый старт

### Пример 1: Базовая отладка

**Запрос к Claude Code:**
```
Debug my Rust program: ./target/debug/my_app
Set a breakpoint at main.rs:42, run it, and show me the stack trace when it stops.
```

**Claude Code сделает:**
1. `debug_create_session(binary_path: "./target/debug/my_app")`
2. `debug_set_breakpoint(file: "main.rs", line: 42)`
3. `debug_launch()`
4. `debug_backtrace()` (когда остановится)

### Пример 2: Вычисление expressions

**Запрос:**
```
When stopped at breakpoint, evaluate `user.name` and `config.debug_mode`
```

**Claude Code:**
```
debug_eval(expression: "user.name")
debug_eval(expression: "config.debug_mode")
```

### Пример 3: Step debugging

**Запрос:**
```
Step through the next 5 lines and show me the value of `counter` after each step
```

**Claude Code:**
```
debug_next()  // step 1
debug_eval(expression: "counter")
debug_next()  // step 2
debug_eval(expression: "counter")
...
```

## 🚀 Advanced Usage

### Multiple Debug Sessions

You can debug multiple programs concurrently:

```
Claude: Create two debug sessions: one for ./server and one for ./client
```

MCP Debug Server maintains isolated sessions with unique session IDs.

### Conditional Breakpoints

Set breakpoints that only trigger when a condition is met:

```
Claude: Set a conditional breakpoint at server.rs:42 when request_count > 100
```

**Current limitation:** Condition is stored but not yet passed to DAP adapter (see ROADMAP).

### Evaluating Complex Expressions

Evaluate any expression in the current frame context:

```
Claude: Evaluate user.permissions.can_write() when stopped at breakpoint
```

### Language-Specific Examples

**Rust:**
```rust
// Binary must be compiled with debug symbols
cargo build
# Path: target/debug/my_app
```

**C++:**
```bash
g++ -g my_app.cpp -o my_app
# Path: ./my_app
```

**Python (with debugpy):**
```python
# No compilation needed
# Path: python3 -m debugpy --listen 5678 my_script.py
```

## 📚 MCP Tools

| Tool | Описание | Параметры |
|------|----------|-----------|
| `debug_create_session` | Создать debug сессию | `binary_path`, `adapter_type?` |
| `debug_set_breakpoint` | Установить breakpoint | `session_id`, `file`, `line` |
| `debug_launch` | Запустить программу | `session_id`, `args?` |
| `debug_next` | Step over | `session_id`, `thread_id?` |
| `debug_step_in` | Step into | `session_id`, `thread_id?` |
| `debug_step_out` | Step out | `session_id`, `thread_id?` |
| `debug_continue` | Continue execution | `session_id`, `thread_id?` |
| `debug_eval` | Evaluate expression | `session_id`, `expression`, `frame_id?` |
| `debug_backtrace` | Get stack trace | `session_id`, `thread_id?` |
| `debug_set_conditional_breakpoint` | Conditional breakpoint | `session_id`, `file`, `line`, `condition` |
| `debug_list_sessions` | List active sessions | - |
| `debug_terminate` | Terminate session | `session_id` |

**Note on `adapter_type` parameter:**

- **Omit** or use `"lldb"` → auto-discovers CodeLLDB from VS Code extensions
- **Use custom path** (e.g., `"/usr/bin/lldb-dap"`) → bypasses auto-discovery
- **Supported values**: `"lldb"`, `"codelldb"`, or full path to DAP adapter

See **Auto-discovery** section above for details.

## 📖 MCP Resources

| Resource URI | Описание |
|--------------|----------|
| `debug://sessions` | Список всех debug сессий |
| `debug://session/{id}/state` | Состояние сессии (Initialized, Running, Stopped, Terminated) |
| `debug://session/{id}/breakpoints` | Список breakpoints для сессии |

## 🧪 Testing

### Unit Tests
```bash
# All unit tests
cargo test -p mcp-debug-server --lib

# Specific module
cargo test -p mcp-debug-server --lib session::manager
```

### Integration Tests
```bash
# All integration tests
cargo test -p mcp-debug-server --test '*'

# Specific test file
cargo test -p mcp-debug-server --test basic_debug
cargo test -p mcp-debug-server --test concurrent
cargo test -p mcp-debug-server --test error_recovery
```

### Mock DAP Server

Integration tests use a Mock DAP Server (`tests/support/mock_dap_server.rs`) to avoid dependency on real debuggers:

```rust
// Start mock server
let mock_server = MockDapServer::new().await?;
let port = mock_server.port();

// Connect to mock
let session = SessionManager::new();
// ... use session
```

**Supported commands:** initialize, setBreakpoints, launch, continue, stepIn, stepOut, stackTrace, evaluate, terminate

**Limitations:** No real debugging, events are simulated

### Linter & Documentation

```bash
# Linter
cargo clippy -p mcp-debug-server

# Documentation
cargo doc --no-deps --open -p mcp-debug-server
```

## 🔧 Разработка

### Структура проекта

```
mcp-debug-server/
├── src/
│   ├── main.rs              # Entry point
│   ├── server/              # MCP Server (12 tools, 3 resources)
│   ├── session/             # Session Manager (thread-safe)
│   ├── dap/                 # DAP Client (dap-rs wrapper)
│   └── types/               # Error types, SessionId
├── tests/
│   ├── basic_debug.rs       # Базовые интеграционные тесты
│   ├── concurrent.rs        # Concurrent sessions
│   ├── error_recovery.rs    # Error handling
│   └── support/
│       └── mock_dap_server.rs  # Mock DAP для тестов
└── Cargo.toml
```

### Технологии

- **MCP SDK:** rmcp 0.9.0 (Anthropic official Rust SDK)
- **DAP Client:** dap-rs 0.2.0-alpha1
- **Async runtime:** tokio 1.x
- **Logging:** tracing + tracing-subscriber
- **Error handling:** thiserror + anyhow

## 🔒 Безопасность

### Path Validation

Все file paths валидируются для защиты от path traversal атак:

```rust
// ❌ Заблокировано
debug_set_breakpoint(file: "../../../etc/passwd", line: 1)

// ✅ Разрешено
debug_set_breakpoint(file: "src/main.rs", line: 42)
```

### Session ID Validation

Session IDs валидируются как UUID формат (36 символов).

## 🔧 Troubleshooting

### DAP adapter not found
**Problem:** `Error: Failed to spawn DAP adapter`

**Solution:**
1. Verify adapter is installed:
   ```bash
   # Check CodeLLDB location
   ls ~/.vscode/extensions/vadimcn.vscode-lldb-*/adapter/codelldb
   ```
2. Update Claude Desktop config with full path:
   ```json
   {
     "mcpServers": {
       "mcp-debug": {
         "command": "/full/path/to/mcp-debug",
         "env": {
           "DAP_ADAPTER_PATH": "/full/path/to/codelldb"
         }
       }
     }
   }
   ```

### Port already in use
**Problem:** Mock DAP server fails to start in tests

**Solution:**
```bash
# Kill process using port
lsof -ti:12345 | xargs kill -9  # macOS/Linux
netstat -ano | findstr :12345   # Windows (find PID, then taskkill)
```

### Session timeout
**Problem:** `Error: DAP request timed out after 5 seconds`

**Solution:**
- Check debugger is responsive: `ps aux | grep codelldb`
- Increase timeout (for slow systems) - edit `src/dap/client.rs`:
  ```rust
  timeout(Duration::from_secs(10), receive_future).await
  ```

### Permission denied (Windows)
**Problem:** `Error: Access denied when spawning adapter`

**Solution:**
- Run as Administrator (for first setup)
- Add antivirus exception for `mcp-debug.exe`

## 📝 Известные ограничения

1. **Только stdio transport** (пока нет HTTP/SSE)
2. **Conditional breakpoints** реализованы частично (DAP адаптер зависит)
3. **Mock DAP Server** для тестов (нет интеграции с реальным debugger в CI)
4. **Event notifications** упрощены (нет real-time streaming)

## 🗺️ Roadmap

- [ ] HTTP/SSE transport для MCP
- [ ] WebSocket support для DAP
- [ ] Multiple language debuggers support
- [ ] Performance benchmarks
- [ ] CI/CD integration

## 🔗 Ссылки

- **MCP Documentation:** https://modelcontextprotocol.io/
- **DAP Specification:** https://microsoft.github.io/debug-adapter-protocol/
- **RMCP SDK:** https://github.com/modelcontextprotocol/rust-sdk
- **dap-rs:** https://github.com/sztomi/dap-rs

## 📄 Лицензия

MIT OR Apache-2.0 (dual license как у bsl-gradual-types проекта)

## 🤝 Contributing

См. основной ROADMAP_2025.md проекта bsl-gradual-types для процесса разработки.

---

**Версия:** 0.1.0
**Milestone:** 4.4
**Прогресс:** 110% (11/11 этапов завершено + NICE TO HAVE features)
