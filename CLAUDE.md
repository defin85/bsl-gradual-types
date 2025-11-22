# CLAUDE.md

AI-ассистент инструкции для BSL Gradual Type System проекта.

## 🤖 Автономное тестирование LSP (ВАЖНО!)

**С 2025-11-12 ты можешь САМОСТОЯТЕЛЬНО тестировать LSP функции!**

### Как тестировать:

1. **Запустить Web API сервер:**
   ```bash
   /start-lsp-api
   # Или вручную:
   cargo run --release -p bsl-backend --bin bsl-web-server -- \
     --port 3002 --enable-cors true \
     --syntax-helper-path examples/syntax_helper
   ```

2. **Использовать endpoints для тестирования:**
   ```bash
   # Тестировать hover
   curl -X POST http://localhost:3002/api/hover/enhanced \
     -H "Content-Type: application/json" \
     -d '{"code":"ТЗ = Новый ТаблицаЗначений;","line":1,"column":0}'

   # Тестировать diagnostics
   curl -X POST http://localhost:3002/api/diagnostics \
     -H "Content-Type: application/json" \
     -d '{"code":"...код BSL..."}'

   # Отладка AST парсинга
   curl -X POST http://localhost:3002/api/debug/ast \
     -H "Content-Type: application/json" \
     -d '{"code":"..."}'
   ```

3. **Итерировать быстро:**
   - Изменить код → Пересобрать → Перезапустить сервер → Протестировать через curl
   - **НЕ нужно** просить пользователя перезапускать VSCode!
   - **5-10x быстрее** итерации

**Доступные endpoints:**
- `POST /api/hover/enhanced` - детальная информация hover
- `POST /api/diagnostics` - синтаксические + семантические ошибки
- `POST /api/debug/ast` - AST дерево и symbol table
- `POST /api/validate` - быстрая валидация (legacy)

**См:** [docs/api/web-api-reference.md](docs/api/web-api-reference.md) для полной документации.

---

## 🐛 Отладка Rust кода через MCP Debug

**Приоритетный метод для глубокой отладки сложных проблем!**

### Когда использовать MCP Debug

**Используй для:**
- ✅ Сложная логика type resolution (пошаговое прохождение алгоритмов)
- ✅ Неожиданное поведение валидации (semantic/syntax diagnostics)
- ✅ Failing интеграционные тесты (анализ state в точке падения)
- ✅ Race conditions, memory issues (глубокая отладка)
- ✅ Изучение внутреннего flow (hover, completion, signature help)

**НЕ используй для:**
- ❌ Быстрое тестирование LSP функций → используй Web API (`/start-lsp-api`)
- ❌ Простые синтаксические ошибки → используй логи и тесты
- ❌ Проверка output → используй unit тесты

### Workflow отладки

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
// Через MCP tool
mcp__mcp-debug__debug_create_session(
  binary_path: "C:\\1CProject\\bsl-gradual-types\\target\\debug\\bsl-lsp-server.exe"
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

### Entry Points для отладки

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

### Важные ограничения и workarounds

⚠️ **Проблема: Breakpoints не верифицируются**
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

⚠️ **Проблема: Conditional breakpoints могут не работать**
```rust
// ❌ Может не работать в CodeLLDB (Windows)
mcp__mcp-debug__debug_set_conditional_breakpoint(
  condition: "variable == value"
)
```
**Workaround:** Используй regular breakpoints + `debug_eval` для проверки условия

⚠️ **Проблема: Async тесты выполняются мгновенно**
- Tokio runtime завершает тесты за <100ms
- Сложно поймать breakpoint

**Решение:** Используй **синхронные примеры** (examples/*.rs) вместо async тестов

⚠️ **Проблема: Долгая инициализация**
- Парсинг `examples/syntax_helper` занимает 10-15 секунд
- Программа может выполниться и завершиться до проверки событий

**Решение:**
```rust
// После debug_continue
sleep 10-15 секунд перед debug_poll_events
```

⚠️ **Проблема: Variables unavailable after lifetime**
```
Error: Variable 'x' not available
```
**Причина:** Rust DWARF behavior - переменные удаляются после scope
**Workaround:** Ставить breakpoints **внутри** нужного scope

### Best Practices

**DO ✅:**
- Всегда использовать **debug builds** (`cargo build` без `--release`)
- Ставить breakpoints на **entry functions** (main, test entry points)
- Использовать `debug_poll_events` для **мониторинга** выполнения
- Давать время на **инициализацию** (sleep после launch/continue)
- Проверять **backtrace** для понимания call stack
- Использовать **простые примеры** (examples/*.rs) для отладки
- Комбинировать с **Web API** для итеративного тестирования

**DON'T ❌:**
- ❌ НЕ использовать release builds (агрессивный инлайнинг)
- ❌ НЕ ставить breakpoints на инлайненные функции
- ❌ НЕ использовать conditional breakpoints без тестирования
- ❌ НЕ забывать про время инициализации
- ❌ НЕ использовать для простого тестирования (Web API быстрее)

### Сравнение с Web API

| Критерий | MCP Debug | Web API |
|----------|-----------|---------|
| **Скорость итерации** | Медленная (5-10 мин) | Быстрая (30 сек) |
| **Глубина отладки** | Полная (state, backtrace) | Поверхностная (output) |
| **Сложность setup** | Средняя | Низкая |
| **Use case** | Сложная логика, bugs | Быстрое тестирование |
| **Итеративность** | Низкая | Высокая |

**Рекомендация:** Начинай с **Web API** для быстрой итерации, переходи на **MCP Debug** когда нужна глубокая отладка.

### Примеры реальной отладки

**Отладка semantic diagnostics:**
```rust
// 1. Соберём тест
cargo test --package bsl-backend --test semantic_diagnostics_lsp_test --no-run

// 2. Создадим сессию
let binary = "target\\debug\\deps\\semantic_diagnostics_lsp_test-HASH.exe"
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
let session = debug_create_session("target\\debug\\examples\\validator_demo.exe")
debug_set_breakpoint(session, "examples/validator_demo.rs", 76)
debug_launch(session)

// Дальше step through
debug_continue(session)
debug_next(session)  // Step over
debug_step_in(session)  // Step into function
```

---

## 📚 Навигация по документации

### 🗺️ Roadmap и прогресс

- **[ROADMAP_2025.md](ROADMAP_2025.md)** — актуальный план развития проекта
- **[ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md)** — архив завершённых Milestones (13 этапов)
- **[docs/guides/roadmap-verification.md](docs/guides/roadmap-verification.md)** — правила проверки выполнения

### 📖 Руководства разработчика

- **[docs/guides/development-workflow.md](docs/guides/development-workflow.md)** — команды cargo/npm/bash, сборка, тестирование
- **[docs/guides/tooling-guide.md](docs/guides/tooling-guide.md)** — MCP инструменты, ast-grep, sourcebot

### 🏗️ Архитектура

- **[docs/architecture/type_system_architecture.md](docs/architecture/type_system_architecture.md)** — система типов + **визуальная диаграмма** (Mermaid)
- **[docs/architecture/milestones-history.md](docs/architecture/milestones-history.md)** — история Milestone 2.8-2.18
- **[docs/architecture/components-detailed.md](docs/architecture/components-detailed.md)** — детальные компоненты

### 🌐 API и интеграция

- **[docs/api/web-api-reference.md](docs/api/web-api-reference.md)** — Web API endpoints с примерами curl

### 📚 Научная база

- **[docs/reference/scientific-basis.md](docs/reference/scientific-basis.md)** — Balyuk & Popova (2021)

### 🎯 Общая документация

- **[docs/README.md](docs/README.md)** — главный навигатор всей документации

---

## 🤖 Автоматизированные навыки (Claude Skills)

Используй Skill tool для автоматизации частых задач:

### Доступные Skills

**Build** — комплексная сборка всех компонентов проекта
```bash
/build
```
Что делает: запускает `build-all.sh` скрипт для автоматической сборки Rust бинарников (LSP, Web, CLI), VSCode Extension, копирования в bin/, сборки WASM, проверки целостности
**Файл:** [.claude/skills/build.md](.claude/skills/build.md)
**Скрипт:** [build-all.sh](build-all.sh)

**Test Runner** — комплексное тестирование проекта
```bash
/test-runner
```
Что делает: Rust unit + integration тесты, TypeScript тесты, compilation checks
**Файл:** [.claude/skills/test-runner.md](.claude/skills/test-runner.md)

**API Tester** — тестирование BSL Web API
```bash
/api-tester
```
Что делает: проверка всех endpoints с URL-encoding для кириллицы
**Файл:** [.claude/skills/api-tester.md](.claude/skills/api-tester.md)

**Roadmap Checker** — автоматическая проверка выполнения Milestone задач
```bash
/roadmap-checker
```
Что делает: grep/Read/cargo test для честной проверки прогресса
**Файл:** [.claude/skills/roadmap-checker.md](.claude/skills/roadmap-checker.md)

**Web UI** — запуск веб-сервера с UI для просмотра типов
```bash
/web-ui
```
Что делает: сборка frontend (WASM через Trunk), копирование статики, запуск веб-сервера с platform types, открытие браузера на http://127.0.0.1:8080
**Файл:** [.claude/skills/web-ui.md](.claude/skills/web-ui.md)

**Test Progress** — тестирование прогресса парсинга (Windows)
```bash
/test-progress
```
Что делает: очистка Windows File System Cache, сборка LSP, копирование в расширение, инструкции для тестирования прогресса парсинга platform types
**Файл:** [.claude/skills/test-progress.md](.claude/skills/test-progress.md)
**Требования:** Windows 10/11, права администратора

**Start LSP API** — запуск Web API для автоматизированного тестирования LSP
```bash
/start-lsp-api
```
Что делает: сборка и запуск bsl-web-server для тестирования LSP функций через HTTP API (POST /api/validate для semantic diagnostics, GET /api/search для типов). Позволяет Claude автоматически тестировать исправления без VSCode
**Файл:** [.claude/skills/start-lsp-api.md](.claude/skills/start-lsp-api.md)
**Порт:** http://localhost:3002

---

## 🎯 Ключевые принципы проекта

### 1. Right-Sized Architecture
**6-8 компонентов** вместо 25-30. Start simple, scale up по необходимости.

### 2. Semantic IR Layer (Milestone 2.8)
**Независимость от парсера** через SemanticProgram:
```
AST → IR (SemanticProgram) → AnalysisEngine → TypeResolver
```

### 3. Честная проверка выполнения
**ОБЯЗАТЕЛЬНО:** grep/Read/cargo test **ПЕРЕД** отчётом о выполнении Milestone задач.
**См.:** [docs/guides/roadmap-verification.md](docs/guides/roadmap-verification.md)

### 4. Модульная документация
Каждая тема в отдельном файле в `docs/`. Легко найти, легко обновить.

### 5. Фасетная система типов
Один тип 1С = множество представлений: Manager | Object | Reference | Selection | List
**Научная основа:** Balyuk & Popova (2021)

---

## ⚠️ Особенности проекта

### GitBash на Windows
- ✅ Используй Unix-style команды (`ls`, `grep`, `find`)
- ❌ НЕ используй PowerShell syntax

### URL-encoding для кириллицы
```bash
# ❌ НЕ работает в GitBash
curl "http://localhost:3002/api/search?q=Массив"

# ✅ Работает
curl "http://localhost:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"
```
**Конвертация:** `python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"`

### 1С проекты НЕ тестируются
**ИСКЛЮЧЕНИЕ:** Проекты НА ПЛАТФОРМЕ 1С (встроенный язык) — НЕ запускать Tester
**Причина:** Нет testing framework для встроенного языка 1С
**Pipeline:** architect → coder → reviewer (без tester)

**НО:** Наш проект (BSL Gradual Types) написан на **Rust/TypeScript** → тестируется полностью!

### Ответы на русском
Всегда используй русский язык в ответах.

---

## 🚀 Быстрый старт

### Сборка и запуск

```bash
# Сборка
cargo build --release

# LSP Server
cargo run --bin bsl-lsp-server

# Web Server (с типами платформы)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper

# Тесты
cargo test --workspace
```

**Детали:** [docs/guides/development-workflow.md](docs/guides/development-workflow.md)

### Проверка Roadmap

```bash
# Автоматическая проверка Milestone
/roadmap-checker

# Или вручную (см. руководство)
```

**Детали:** [docs/guides/roadmap-verification.md](docs/guides/roadmap-verification.md)

---

## 📁 Структура документации

```
bsl-gradual-types/
├── CLAUDE.md                    # 🎯 Этот файл (навигатор)
├── ROADMAP_2025.md              # Актуальный roadmap
├── ROADMAP_ARCHIVE_2025.md      # Архив Milestones
│
├── .claude/
│   └── skills/                  # Автоматизированные навыки
│       ├── build.md             # Сборка всех компонентов
│       ├── test-runner.md       # Тестирование
│       ├── api-tester.md        # API тестирование
│       └── roadmap-checker.md   # Проверка Milestone
│
└── docs/
    ├── README.md                # Главный навигатор
    │
    ├── guides/                  # Практические руководства
    │   ├── development-workflow.md
    │   ├── roadmap-verification.md
    │   └── tooling-guide.md
    │
    ├── architecture/            # Архитектурные описания
    │   ├── type_system_architecture.md
    │   ├── milestones-history.md
    │   └── components-detailed.md
    │
    ├── api/                     # API документация
    │   └── web-api-reference.md
    │
    └── reference/               # Справочные материалы
        └── scientific-basis.md
```

---

## 🔗 Полезные ссылки

- **Проект на GitHub:** (добавь URL если есть)
- **Научная статья:** [Balyuk & Popova (2021)](https://ceur-ws.org/Vol-2984/paper13.pdf)
- **MCP Documentation:** https://modelcontextprotocol.io/
- **Claude Code Docs:** https://docs.claude.com/en/docs/claude-code/

---

## 💡 Философия

**Реальный прогресс вместо иллюзии выполнения.**

- Честная оценка прогресса с доказательствами
- Модульная документация для простоты поддержки
- Автоматизация через Claude Skills
- Right-Sized Architecture: простота масштабируется лучше сложности

---

**Версия проекта:** 0.4.0
**Прогресс Версии 2.0:** ~65% завершено (13/20 Milestones)
**Последнее обновление документации:** 2025-11-03
