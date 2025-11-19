# MCP Debug Server Architecture

## Overview

MCP Debug Server is a **Model Context Protocol (MCP) server** that provides AI assistants with interactive debugging capabilities through the **Debug Adapter Protocol (DAP)**.

### High-Level Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      Claude Code (AI)                         │
│                    (MCP Client)                               │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     │ JSON-RPC over stdio
                     │
┌────────────────────▼─────────────────────────────────────────┐
│                 MCP Debug Server                              │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │           MCP Server Layer (rmcp 0.9.0)              │    │
│  │  - 12 Tools (debug_create_session, etc.)             │    │
│  │  - 3 Resources (sessions, state, breakpoints)        │    │
│  └──────────────────┬───────────────────────────────────┘    │
│                     │                                         │
│  ┌──────────────────▼───────────────────────────────────┐    │
│  │        Session Management Layer                      │    │
│  │  SessionManager: Arc<RwLock<HashMap<Id, Session>>>   │    │
│  │  - Concurrent access (read/write locks)              │    │
│  │  - State machine (Initialized → Running → Stopped)   │    │
│  └──────────────────┬───────────────────────────────────┘    │
│                     │                                         │
│  ┌──────────────────▼───────────────────────────────────┐    │
│  │          DAP Client Layer                            │    │
│  │  DapClient:                                          │    │
│  │  - EventRouter (async message routing)              │    │
│  │  - EventProcessor (stopped/output/terminated)       │    │
│  │  - EventBuffer (Arc<Mutex<HashMap>>)                │    │
│  │  - Timeout protection (5 seconds)                   │    │
│  └──────────────────┬───────────────────────────────────┘    │
│                     │                                         │
└─────────────────────┼─────────────────────────────────────────┘
                      │
                      │ DAP Protocol (stdio)
                      │
┌─────────────────────▼─────────────────────────────────────────┐
│              DAP Server (Language-Specific)                    │
│  - CodeLLDB (Rust/C/C++)                                      │
│  - GDB (C/C++/Rust)                                           │
│  - debugpy (Python)                                           │
│  - node-debug (JavaScript/TypeScript)                         │
└────────────────────┬───────────────────────────────────────────┘
                     │
                     │ Native Debugger Protocol
                     │
┌────────────────────▼───────────────────────────────────────────┐
│                Native Debugger                                 │
│  - LLDB (for CodeLLDB)                                         │
│  - GDB (for GDB adapter)                                       │
└────────────────────────────────────────────────────────────────┘
```

---

## Component Details

### 1. MCP Server Layer

**Responsibilities:**
- Expose debugging capabilities as MCP tools
- Handle MCP protocol (JSON-RPC 2.0)
- Validate parameters
- Format responses for AI consumption

**Implementation:**
- Uses `rmcp` crate (official Anthropic Rust SDK)
- Tools implemented with `#[tool]` macro
- Resources implemented with `list_resources()` / `read_resource()`

**Key Files:**
- `src/server/mod.rs` - MCP server initialization
- `src/server/tools.rs` - 12 MCP tools definitions
- `src/server/resources.rs` - 3 MCP resources

**Tools (12 total):**
1. `debug_create_session` - Create debug session
2. `debug_set_breakpoint` - Set breakpoint at file:line
3. `debug_launch` - Launch program
4. `debug_next` - Step over (next line)
5. `debug_step_in` - Step into function
6. `debug_step_out` - Step out of function
7. `debug_continue` - Continue execution
8. `debug_eval` - Evaluate expression
9. `debug_backtrace` - Get stack trace
10. `debug_set_conditional_breakpoint` - Conditional breakpoint
11. `debug_list_sessions` - List active sessions
12. `debug_terminate` - Terminate session

**Resources (3 total):**
1. `debug://sessions` - List all sessions
2. `debug://session/{id}/state` - Session state
3. `debug://session/{id}/breakpoints` - Breakpoints for session

---

### 2. Session Management Layer

**Responsibilities:**
- Manage lifecycle of debug sessions
- Ensure thread-safe concurrent access
- Track session state
- Handle session cleanup

**Data Structure:**
```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, DebugSession>>>,
}

pub struct DebugSession {
    pub id: SessionId,
    pub dap_client: DapClient,
    pub binary_path: String,
    pub state: SessionState,
    pub current_thread_id: Arc<Mutex<Option<u32>>>,
    pub breakpoints: HashMap<String, Vec<u32>>,
}

pub enum SessionState {
    Initialized,  // Ready to launch
    Running,      // Executing
    Stopped,      // Paused on breakpoint
    Terminated,   // Finished
}
```

**Thread Safety Model:**

```
┌─────────────────────────────────────────────────┐
│        SessionManager (shared)                  │
│  Arc<RwLock<HashMap<SessionId, DebugSession>>>  │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┴──────────┐
        │                    │
     Read Lock           Write Lock
        │                    │
   ┌────▼────┐        ┌──────▼─────┐
   │ List    │        │ Create     │
   │ Sessions│        │ Session    │
   │         │        │            │
   │ Session │        │ Terminate  │
   │ Exists  │        │ Session    │
   └─────────┘        └────────────┘
```

**Lock Hierarchy:**
1. `SessionManager` RwLock (outermost)
2. `EventBuffer` Mutex (innermost)

**Deadlock Prevention:**
- Always acquire locks in same order
- Release locks as soon as possible
- No nested lock acquisitions

**Key Files:**
- `src/session/manager.rs` - SessionManager implementation
- `src/session/state.rs` - State machine
- `src/types/session_id.rs` - SessionId type

---

### 3. DAP Client Layer

**Responsibilities:**
- Communicate with DAP adapter via stdio
- Route DAP messages (requests → responses, events → handlers)
- Process events asynchronously
- Buffer events for polling

**Architecture (Full Async):**

```
DapClient
├── spawn() → launches adapter process
├── send_request() → sends DAP request with timeout
│   ├── Registers oneshot channel in response_map
│   ├── Sends JSON via DapWriter
│   └── Awaits response via oneshot (5s timeout)
│
├── EventRouter (background task)
│   ├── Reads from DapReader (stdio)
│   ├── Routes responses → oneshot channels
│   └── Routes events → event_tx channel
│
└── EventProcessor (background task)
    ├── Receives events from event_rx
    ├── Handles stopped/output/terminated/continued
    └── Adds to EventBuffer (Arc<Mutex<HashMap>>)
```

**Event Flow:**

```
DAP Adapter (CodeLLDB)
     │
     │ (sends event via stdout)
     ▼
DapReader::receive()
     │
     ▼
EventRouter::run()  ← background task
     │
     ├─→ Response? → oneshot_tx.send()
     │
     └─→ Event? → event_tx.send()
              │
              ▼
        EventProcessor::run()  ← background task
              │
              ├─→ stopped → handle_stopped_event()
              ├─→ output → handle_output_event()
              ├─→ terminated → handle_terminated_event()
              └─→ continued → handle_continued_event()
                    │
                    ▼
              EventBuffer::add_to_buffer()
                    │
                    ▼
              Arc<Mutex<HashMap<SessionId, Vec<Event>>>>
```

**Timeout Handling:**

```rust
// In send_request():
let receive_future = async {
    // Wait for response via oneshot
};

match timeout(Duration::from_secs(5), receive_future).await {
    Ok(result) => result,
    Err(_) => {
        // Cleanup: remove from response_map
        response_map.remove(&seq);
        Err(DapError::Timeout)
    }
}
```

**Key Files:**
- `src/dap/client.rs` - DapClient main logic
- `src/dap/router.rs` - EventRouter (message routing)
- `src/dap/events.rs` - EventProcessor + EventBuffer
- `src/dap/transport.rs` - DapWriter/DapReader (stdio)
- `src/dap/protocol.rs` - DAP message types

**Adapter Auto-Discovery:**

The `src/config/adapters.rs` module provides automatic discovery of DAP adapters:

```rust
pub fn find_codelldb() -> Option<PathBuf> {
    // Searches ~/.vscode/extensions/vadimcn.vscode-lldb-*
    // Returns full path to codelldb executable if found
}

pub fn resolve_adapter(adapter_type: &str) -> String {
    match adapter_type {
        "lldb" | "codelldb" => {
            find_codelldb()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| adapter_type.to_string())
        }
        _ => adapter_type.to_string(), // Pass through custom paths
    }
}
```

**Usage:**
- When `debug_create_session` receives `adapter_type: "lldb"` → auto-discovers CodeLLDB
- If CodeLLDB not found → falls back to `"lldb"` (assumes in PATH)
- Custom paths (e.g., `"/usr/bin/lldb-dap"`) → used as-is

**Testing:**
- Integration test: `tests/adapter_autodiscovery.rs` verifies discovery → spawn → initialize chain

---

## Error Handling Strategy

### Error Types Hierarchy

```
McpDebugError (top-level)
├── DapProtocol(String)      → DapError
├── SessionNotFound(String)
├── InvalidState { ... }
├── Io(std::io::Error)       → std::io::Error
├── Json(serde_json::Error)  → serde_json::Error
├── Timeout
├── AdapterCrashed
├── InvalidParams(String)
└── Other(String)            → anyhow::Error (downcast)
```

### Conversion Chain

```
DapError → McpDebugError → rmcp::ErrorData
           (via From)       (via From)
```

### Error Propagation

```
Tool Call (MCP)
    │
    └─→ SessionManager method
            │
            └─→ with_session()
                    │
                    └─→ DapClient method
                            │
                            └─→ DapError
                                    │
                                    ▼
                            McpDebugError (From)
                                    │
                                    ▼
                            rmcp::ErrorData (From)
                                    │
                                    ▼
                            MCP Response Error
```

### Graceful Degradation

**Crashed Session Handling:**
```rust
// In SessionManager::with_session():
if let Err(ref e) = result {
    let err_str = e.to_string();
    if err_str.contains("IO error") || err_str.contains("Broken pipe") {
        tracing::error!("DAP client I/O error detected, marking session as terminated");
        let _ = session.set_state(SessionState::Terminated);
    }
}
```

**Event Processing Errors:**
- Malformed events → logged, skipped (not added to buffer)
- EventRouter shutdown → graceful task termination
- EventProcessor shutdown → graceful task termination

---

## Testing Strategy

### 1. Unit Tests (27 tests)

**Coverage:**
- `session::state` - state transitions (3 tests)
- `session::manager` - creation, validation (2 tests)
- `types::session_id` - creation, display (4 tests)
- `types::error` - error conversions (5 tests)
- `dap::events` - event handling, counters (7 tests)
- `server::tools` - formatting helpers (2 tests)
- `server::resources` - resource reading (4 tests)

**Run:**
```bash
cargo test -p mcp-debug-server --lib
```

---

### 2. Integration Tests (72 tests)

**Test Files:**

1. **basic_debug.rs** (9 tests)
   - Session lifecycle
   - State transitions
   - Mock DAP protocol

2. **concurrent.rs** (10 tests)
   - Concurrent session creation
   - Concurrent operations
   - Race conditions
   - Session isolation
   - Stress test (1000 parallel reads)

3. **error_recovery.rs** (16 tests)
   - Nonexistent sessions
   - Invalid state transitions
   - Multiple terminate attempts
   - Session ID edge cases
   - State transition matrix

4. **event_routing.rs** (10 tests)
   - EventRouter message routing
   - Response → oneshot channels
   - Events → event_tx channel
   - Mixed messages
   - Orphaned responses

5. **event_processing.rs** (11 tests)
   - EventProcessor event handling
   - EventBuffer polling
   - Concurrent access
   - Malformed events
   - Overflow protection

6. **full_async_integration.rs** (16 tests)
   - Full async debug cycle
   - Event isolation between sessions
   - Concurrent polling
   - State transitions with events
   - Cleanup after terminate

**Mock DAP Server:**

Integration tests use `tests/support/mock_dap_server.rs` - a TCP-based mock that simulates DAP protocol without requiring real debugger.

**Supported commands:**
- initialize, setBreakpoints, launch
- continue, next, stepIn, stepOut
- stackTrace, evaluate, terminate

**Run:**
```bash
cargo test -p mcp-debug-server --test '*'
```

---

### 3. Test Coverage Estimation

**High Coverage (>80%):**
- EventRouter: ~90%
- EventProcessor: ~85%
- EventBuffer: ~90%
- SessionState: ~95%

**Medium Coverage (50-80%):**
- SessionManager: ~70%
- DapClient: ~60%

**Lower Coverage (<50%):**
- Full end-to-end with real adapter: ~30% (requires refactoring)

---

## Performance Considerations

### Concurrency

**Read-Heavy Workload:**
- `SessionManager` uses RwLock → multiple concurrent reads
- No blocking on read operations

**Write Operations:**
- Session creation: acquires write lock briefly
- Termination: acquires write lock + cleanup

**Event Processing:**
- Lock-free counters: `AtomicU64` (Relaxed ordering)
- EventBuffer: mutex only for HashMap access (short critical section)

### Memory Usage

**Session Storage:**
- `HashMap<SessionId, DebugSession>` - O(n) where n = active sessions
- Each session: ~2 KB (DapClient + state + breakpoints)

**EventBuffer:**
- `HashMap<SessionId, Vec<Event>>` - O(n*m) where m = events per session
- **Recommendation:** Add size limit (1000 events/session) to prevent memory leak

### Latency

**DAP Request:**
- Network: <1ms (local process via stdio)
- Timeout: 5 seconds (configurable)
- Typical: 10-50ms (adapter processing)

**MCP Tool Call:**
- Overhead: <5ms (JSON parsing + routing)
- Total: 15-60ms (DAP + MCP overhead)

---

## Security

### Path Validation

All file paths validated to prevent path traversal:

```rust
// ❌ Blocked
debug_set_breakpoint(file: "../../../etc/passwd", line: 1)

// ✅ Allowed
debug_set_breakpoint(file: "src/main.rs", line: 42)
```

### Session ID Validation

- UUID v4 format enforced (36 characters)
- No SQL injection risk (not used in queries)

### Process Isolation

- Each DAP adapter spawned as separate process
- No shared memory between sessions
- Adapter crash does not affect other sessions

---

## Future Improvements

### Short-term

1. **EventBuffer size limit**
   - Add 1000 events/session limit
   - Implement LRU eviction

2. **Conditional breakpoints**
   - Extend `DapClient::set_breakpoints()` to pass condition to adapter
   - Update `debug_set_conditional_breakpoint` tool

3. **HTTP/SSE transport**
   - Add HTTP server for MCP (alternative to stdio)
   - Server-Sent Events for real-time event streaming

### Long-term

1. **Multiple language debuggers**
   - Test with debugpy (Python)
   - Test with node-debug (JavaScript)
   - Adapter-specific quirks handling

2. **Performance benchmarks**
   - Measure tool call latency
   - Memory usage profiling
   - Concurrent session stress test (100+ sessions)

3. **Advanced features**
   - Watch expressions
   - Data breakpoints
   - Reverse debugging (if adapter supports)

---

## References

- **DAP Specification:** https://microsoft.github.io/debug-adapter-protocol/
- **RMCP SDK:** https://github.com/modelcontextprotocol/rust-sdk
- **dap-rs:** https://github.com/sztomi/dap-rs
- **CodeLLDB:** https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb

---

**Version:** 1.0
**Date:** 2025-11-19
**Milestone:** 4.4
