# ИИ-инструменты для отладки программ через GDB/LLDB и DAP

## Основные инструменты

### 1. ChatDBG
**ИИ-ассистент с прямой интеграцией в отладчики**
- Поддержка: GDB, LLDB, Pdb, WinDBG
- Функции: диалоговая отладка, автоанализ ошибок, управление отладчиком через LLM
- Установка: `pip install chatdbg`
- GitHub: https://github.com/plasma-umass/ChatDBG

### 2. MCP DAP Server
**Универсальный мост между ИИ и отладчиками (Model Context Protocol)**
- Поддержка: Delve (Go), debugpy (Python), lldb-dap (C/C++), GDB и любые DAP-совместимые отладчики
- Функции: управление breakpoints, контроль выполнения, инспекция переменных, оценка выражений
- GitHub: https://github.com/modelcontextprotocol/servers
- Документация: https://modelcontextprotocol.info

### 3. Debugger MCP
**Rust-based MCP сервер мультиязычной поддержкой**
- Языки: Python, Ruby, Node.js, Go, Rust
- Возможности: полная DAP функциональность для всех языков
- GitHub: https://github.com/modelcontextprotocol/debugger-mcp

### 4. dap-mcp (Python)
**Специализированный DAP-мост для Python**
- Поддержка debugpy и pytest
- Машиночитаемые JSON-отчеты о тестах
- GitHub: https://github.com/markomanninen/mcp-debugpy

### 5. Delve DAP MCP Server (Go)
**MCP-сервер для отладки Go приложений**
- Поддержка: debug, attach, exec, test, core dumps, replay debugging
- GitHub: https://github.com/go-delve/mcp-dap-server
- Документация: https://github.com/go-delve/delve

## DAP-серверы и адаптеры

### lldb-dap
**Официальный DAP-сервер от LLVM**
- Языки: C, C++, Objective-C, Swift
- Установка: `brew install llvm` (macOS) или `apt install lldb` (Linux)
- Документация: https://lldb.llvm.org

### debugpy
**Официальный DAP-адаптер для Python (Microsoft)**
- Режимы: launch, attach, remote debugging
- PyPI: https://pypi.org/project/debugpy/

### GDBServer
**Контрольная программа для удаленной отладки через GDB**
- Документация: https://sourceware.org/gdb/current/onlinedocs/gdb/Server.html

## Интеграция с редакторами

### MCP Debug Tools
**VSCode расширение с DAP поддержкой**
- Функции: управление breakpoints, контроль выполнения, инспекция переменных
- GitHub: https://github.com/hwanyong/mcp-debug-tools

### nvim-dap
**DAP-клиент для Neovim**
- Поддерживает: lldb-dap, debugpy, Delve, CodeLLDB, Node Debug 2
- GitHub: https://github.com/mfussenegger/nvim-dap
- Документация: https://github.com/mfussenegger/nvim-dap/wiki

### CodeLLDB
**LLDB расширение для VSCode**
- GitHub: https://github.com/vadimcn/codelldb

## Инструменты отладки MCP-серверов

### MCP Inspector
**Официальный инструмент для тестирования MCP-серверов**
- Использование: `npx @modelcontextprotocol/inspector <path-to-server>`
- Документация: https://github.com/modelcontextprotocol/inspector

## Быстрый старт

**Для Python отладки через ИИ:**
```bash
pip install debugpy
# Используй dap-mcp или AI Python Debugger MCP
```

**Для Go отладки через ИИ:**
```bash
# Используй Delve DAP MCP Server
go install github.com/go-delve/delve/cmd/dlv@latest
```

**Для C/C++ отладки через ИИ:**
```bash
pip install chatdbg
# или используй lldb-dap/CodeLLDB через MCP DAP Server
```

## Ресурсы

- [Model Context Protocol](https://modelcontextprotocol.io)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [ChatDBG arXiv](https://arxiv.org/abs/2402.00949)
- [MCP Market](https://mcpmarket.com)
- [Glama MCP Tools](https://glama.ai)
