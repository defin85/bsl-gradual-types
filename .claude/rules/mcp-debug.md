---
paths: "**/*.rs"
---

# Отладка Rust кода через MCP Debug

**Приоритетный метод для глубокой отладки сложных проблем!**

## Когда использовать MCP Debug

**Используй для:**
- Сложная логика type resolution (пошаговое прохождение алгоритмов)
- Неожиданное поведение валидации (semantic/syntax diagnostics)
- Failing интеграционные тесты (анализ state в точке падения)
- Race conditions, memory issues (глубокая отладка)
- Изучение внутреннего flow (hover, completion, signature help)

**НЕ используй для:**
- Быстрое тестирование LSP функций → используй Web API (`/start-lsp-api`)
- Простые синтаксические ошибки → используй логи и тесты
- Проверка output → используй unit тесты

## Workflow отладки

**1. Собрать debug build:**
```bash
# LSP Server
cargo build -p bsl-backend --bin bsl-lsp-server

# Test binary
cargo test --package bsl-backend --test semantic_diagnostics_lsp_test --no-run

# Example
cargo build --example validator_demo
```

**2. Создать debug сессию:**
```rust
mcp__mcp-debug__debug_create_session(
  binary_path: "target/debug/bsl-lsp-server"
)
// Вернёт: session_id
```

**3. Установить breakpoints:**
```rust
// Entry points для diagnostics
mcp__mcp-debug__debug_set_breakpoint(
  session_id: "...",
  file: "backend/src/bin/lsp_server.rs",
  line: 736  // did_open - entry point
)

// Semantic validation
mcp__mcp-debug__debug_set_breakpoint(
  file: "backend/src/bin/lsp_server.rs",
  line: 800  // validate_semantics call
)
```

**4. Запустить и отлаживать:**
```rust
// Запуск
mcp__mcp-debug__debug_launch(session_id, args: ["--test-arg"])

// Continue до breakpoint
mcp__mcp-debug__debug_continue(session_id)

// Проверить события
mcp__mcp-debug__debug_poll_events(session_id)

// Backtrace
mcp__mcp-debug__debug_backtrace(session_id)

// Eval переменных
mcp__mcp-debug__debug_eval(session_id, expression: "variable_name")

// Step over/into/out
mcp__mcp-debug__debug_next(session_id)
mcp__mcp-debug__debug_step_in(session_id)
mcp__mcp-debug__debug_step_out(session_id)
```

## Entry Points для отладки

**Diagnostics flow** (`backend/src/bin/lsp_server.rs`):
- `did_open:736` - LSP notification entry point
- `parse_and_validate:764` - PHASE 1: Syntax validation
- `validate_semantics:800` - PHASE 2: Semantic validation
- `syntax_errors_to_diagnostics:172` - Конвертация syntax errors → LSP Diagnostic
- `semantic_error_to_diagnostic:205` - Конвертация semantic errors → LSP Diagnostic
- `publish_diagnostics:821` - Отправка в VSCode

**Type Resolution flow** (`shared/src/engine/analysis_engine.rs`):
- `infer_type` - entry point для вывода типов
- `resolve_type_reference` - резолвинг ссылок на типы

**Тесты для отладки:**
- `backend/tests/semantic_diagnostics_lsp_test.rs` - semantic validation
- `backend/tests/lsp_diagnostics_edge_cases_test.rs` - syntax diagnostics
- `examples/validator_demo.rs` - простой синхронный пример

## Важные ограничения и workarounds

### Breakpoints не верифицируются
```
Event: breakpoint
"verified": false
"message": "Resolved locations: 0"
```

**Причины:**
- Инлайнинг функций (даже в debug builds Rust агрессивно инлайнит)
- Функция не вызывается в текущем execution path
- Оптимизации LLVM

**Решения:**
1. Ставить breakpoints на **top-level functions** (main, test functions)
2. Использовать **#[inline(never)]** attribute для критичных функций
3. Проверять backtrace для понимания реального call stack

### Conditional breakpoints могут не работать
```rust
// Может не работать в CodeLLDB
mcp__mcp-debug__debug_set_conditional_breakpoint(
  condition: "variable == value"
)
```
**Workaround:** Используй regular breakpoints + `debug_eval` для проверки условия

### Async тесты выполняются мгновенно
- Tokio runtime завершает тесты за <100ms
- Сложно поймать breakpoint

**Решение:** Используй **синхронные примеры** (examples/*.rs) вместо async тестов

### Долгая инициализация
- Парсинг `examples/syntax_helper` занимает 10-15 секунд
- Программа может выполниться и завершиться до проверки событий

**Решение:** После `debug_continue` — sleep 10-15 секунд перед `debug_poll_events`

### Variables unavailable after lifetime
```
Error: Variable 'x' not available
```
**Причина:** Rust DWARF behavior - переменные удаляются после scope
**Workaround:** Ставить breakpoints **внутри** нужного scope

## Best Practices

**DO:**
- Всегда использовать **debug builds** (`cargo build` без `--release`)
- Ставить breakpoints на **entry functions** (main, test entry points)
- Использовать `debug_poll_events` для **мониторинга** выполнения
- Давать время на **инициализацию** (sleep после launch/continue)
- Проверять **backtrace** для понимания call stack
- Использовать **простые примеры** (examples/*.rs) для отладки
- Комбинировать с **Web API** для итеративного тестирования

**DON'T:**
- НЕ использовать release builds (агрессивный инлайнинг)
- НЕ ставить breakpoints на инлайненные функции
- НЕ использовать conditional breakpoints без тестирования
- НЕ забывать про время инициализации
- НЕ использовать для простого тестирования (Web API быстрее)

## Сравнение с Web API

| Критерий | MCP Debug | Web API |
|----------|-----------|---------|
| **Скорость итерации** | Медленная (5-10 мин) | Быстрая (30 сек) |
| **Глубина отладки** | Полная (state, backtrace) | Поверхностная (output) |
| **Сложность setup** | Средняя | Низкая |
| **Use case** | Сложная логика, bugs | Быстрое тестирование |
| **Итеративность** | Низкая | Высокая |

**Рекомендация:** Начинай с **Web API** для быстрой итерации, переходи на **MCP Debug** когда нужна глубокая отладка.

## Примеры реальной отладки

**Отладка semantic diagnostics:**
```rust
// 1. Соберём тест
cargo test --package bsl-backend --test semantic_diagnostics_lsp_test --no-run

// 2. Создадим сессию
let binary = "target/debug/deps/semantic_diagnostics_lsp_test-HASH"
let session = debug_create_session(binary)

// 3. Breakpoints на ключевые точки
debug_set_breakpoint(session, "backend/tests/semantic_diagnostics_lsp_test.rs", 281)

// 4. Запуск конкретного теста
debug_launch(session, args: ["test_validate_parameter_type_mismatch", "--exact"])

// 5. Мониторинг
sleep(10) // Даём время на инициализацию
debug_poll_events(session)
debug_backtrace(session)
debug_eval(session, "diagnostics")
```

**Отладка простого примера:**
```rust
// 1. Соберём пример
cargo build --example validator_demo

// 2. Отладка
let session = debug_create_session("target/debug/examples/validator_demo")
debug_set_breakpoint(session, "examples/validator_demo.rs", 76)
debug_launch(session)

// Дальше step through
debug_continue(session)
debug_next(session)  // Step over
debug_step_in(session)  // Step into function
```
