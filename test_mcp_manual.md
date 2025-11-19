# Ручное тестирование MCP Debug Server

## Шаг 1: Запустить MCP сервер

```bash
cd C:\1CProject\bsl-gradual-types-milestone-4.4
set RUST_LOG=mcp_debug_server=debug
.\target\release\mcp-debug.exe
```

## Шаг 2: Отправить JSON-RPC команду через stdin

### Пример 1: Initialize
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "test-client",
      "version": "1.0.0"
    }
  }
}
```

### Пример 2: List Tools
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

### Пример 3: Create Debug Session
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "debug_create_session",
    "arguments": {
      "binary_path": "C:\\1CProject\\bsl-gradual-types-milestone-4.4\\test_debug_program.exe",
      "adapter_type": "C:\\Users\\Egor\\.vscode\\extensions\\vadimcn.vscode-lldb-1.11.8\\adapter\\codelldb.exe"
    }
  }
}
```

## Примечание

Для автоматического тестирования лучше использовать интеграцию с Claude Desktop (см. инструкцию выше).
