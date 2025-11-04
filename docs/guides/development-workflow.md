# Development Workflow Guide

Руководство по командам разработки BSL Gradual Types проекта.

## 📦 Сборка проекта

### Полная сборка workspace

```bash
# Все крейты в режиме разработки
cargo build

# Релизная сборка с оптимизациями
cargo build --release

# Быстрая разработка (opt-level = 1)
cargo build --profile dev-fast
```

### Сборка отдельных компонентов

```bash
# Backend (LSP + Web Server)
cargo build -p bsl-backend --release

# Frontend (Leptos WASM)
cargo build -p bsl-frontend --release

# CLI инструменты
cargo build -p bsl-cli --release

# Shared библиотека
cargo build -p bsl-shared
```

### Бинарные файлы

```bash
# LSP Server
cargo build -p bsl-backend --bin bsl-lsp-server --release

# Web Server
cargo build -p bsl-backend --bin bsl-web-server --release

# CLI
cargo build -p bsl-cli --bin bsl-type-check --release
```

**Расположение:** `target/release/` (или `target/debug/` для dev сборок)

---

## 🧪 Тестирование

### Unit тесты

```bash
# Все тесты в workspace
cargo test

# С детальным выводом
cargo test -- --nocapture

# Конкретный крейт
cargo test -p bsl-backend
cargo test -p bsl-shared
cargo test -p bsl-frontend
```

### Integration тесты

```bash
# Все integration тесты
cargo test --test '*'

# Конкретные тесты
cargo test --test inline_scope_analysis_test
cargo test --test hover_with_spans_test
cargo test --test hover_unknown_type_test
cargo test --test syntax_error_detection_test
cargo test --test semantic_visualization_test
cargo test --test config_parser_guided_test
```

### Тесты по категориям

```bash
# Semantic IR тесты
cargo test -p bsl-shared ir

# Type Resolution тесты
cargo test -p bsl-shared resolver

# Tree-Sitter тесты
cargo test -p bsl-backend tree_sitter

# Parser тесты
cargo test -p bsl-backend parser
```

### VSCode Extension тесты

```bash
cd vscode-extension

# Установка зависимостей (первый раз)
npm install

# Компиляция TypeScript
npm run compile

# Линтинг
npm run lint

# Запуск тестов
npm test

cd ..
```

---

## 🔍 Линтинг и форматирование

### Clippy (Rust линтер)

```bash
# Все workspace
cargo clippy --workspace --all-targets --all-features

# Только backend
cargo clippy -p bsl-backend

# Фикс автоматических проблем
cargo clippy --fix --allow-dirty
```

### Форматирование кода

```bash
# Форматирование всего workspace
cargo fmt

# Проверка без изменений
cargo fmt --check

# Конкретный крейт
cargo fmt -p bsl-backend
```

### Проверка компиляции

```bash
# Быстрая проверка без сборки
cargo check --workspace

# С примерами и тестами
cargo check --workspace --all-targets
```

---

## 🚀 Запуск компонентов

### LSP Server

```bash
# Запуск напрямую
cargo run --bin bsl-lsp-server

# С логированием
RUST_LOG=debug cargo run --bin bsl-lsp-server

# Релизная версия
cargo run --bin bsl-lsp-server --release
```

**Логи:** `rust_lsp_server.log` в текущей директории

### Web Server

```bash
# Базовый запуск (только примитивные типы)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true

# С полными типами платформы (Syntax Helper)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

**Доступен на:** http://127.0.0.1:3002

**Endpoints:**
- `GET /api/health` — статус сервера
- `GET /api/types?search=<query>` — поиск типов
- `POST /api/analyze` — анализ кода
- `GET /api/semantic/:file_path` — semantic visualization

### CLI инструменты

```bash
# Проверка типов
cargo run --bin bsl-type-check -- "Справочники.Контрагенты"

# Автодополнение
cargo run --bin bsl-type-check -- --complete "Справочники."

# Помощь
cargo run --bin bsl-type-check -- --help
```

### Frontend (WASM)

```bash
cd frontend

# Установка Trunk (первый раз)
cargo install trunk

# Dev сервер с hot reload
trunk serve --open

# Релизная сборка
trunk build --release

cd ..
```

**Доступен на:** http://127.0.0.1:8080

**Интеграция:** Frontend интегрирован в Web Server (`bsl-web-server` отдает статические WASM файлы)

---

## 🔧 VSCode Extension

### Разработка

```bash
cd vscode-extension

# Установка зависимостей
npm install

# Компиляция TypeScript (watch режим)
npm run watch

# Компиляция один раз
npm run compile
```

### Упаковка и установка

```bash
cd vscode-extension

# Установка VSCE (первый раз)
npm install -g @vscode/vsce

# Упаковка в .vsix
vsce package

# Установка расширения
code --install-extension bsl-gradual-types-1.0.0.vsix

cd ..
```

### Отладка

1. Открой `vscode-extension/` в VSCode
2. Нажми `F5` для запуска Extension Development Host
3. Открой BSL файл для тестирования

---

## 🌐 Web API тестирование

### Health Check

```bash
curl -s "http://127.0.0.1:3002/api/health" | jq '.'
```

### Поиск типов (латиница)

```bash
curl -s "http://127.0.0.1:3002/api/types?search=Array" | jq '.'
```

### Поиск типов (кириллица)

⚠️ **GitBash требует URL-encoding для кириллицы!**

```bash
# Конвертация через Python
python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"
# Вывод: %D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2

# Использование URL-encoded строки
curl -s "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2" | jq '.'
```

### Анализ кода

```bash
curl -X POST "http://127.0.0.1:3002/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"code": "Функция Тест() Возврат 42; КонецФункции"}' | jq '.'
```

### Semantic Visualization

```bash
# JSON формат
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=json" | jq '.'

# HTML формат
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=html&theme=dark" > semantic_tree.html
start semantic_tree.html
```

**См. также:** [Web API Reference](../api/web-api-reference.md) для полного списка endpoints

---

## 🔬 Примеры и демо

### Запуск примеров

```bash
# Syntax Helper Parser Demo
cargo run --example syntax_helper_parser_demo

# Type Hierarchy Visualization
cargo run --example visualize_parser_v3

# Simple тест
cargo run --example test_simple
```

### Configuration Discovery

```bash
# Парсинг конфигураций 1С
cargo run --example config_discovery -- --path /path/to/configuration
```

---

## 📊 Производительность

### Бенчмарки

```bash
# Все бенчмарки
cargo bench

# Конкретный бенчмарк
cargo bench --bench type_resolution_bench

# Проверка без запуска
cargo bench --no-run
```

### Профилирование

```bash
# Установка profiler (первый раз)
cargo install cargo-flamegraph

# Flamegraph для LSP Server
cargo flamegraph --bin bsl-lsp-server

# Профилирование Web Server
cargo flamegraph --bin bsl-web-server -- --port 3002
```

### Performance Profiler

```bash
# Запуск бенчмарка (10 итераций)
cargo run --bin bsl-profiler benchmark --iterations 10

# С детальным выводом
RUST_LOG=info cargo run --bin bsl-profiler benchmark --iterations 5
```

---

## 🐛 Отладка

### Логирование

```bash
# Включить DEBUG логи
RUST_LOG=debug cargo run --bin bsl-lsp-server

# Только конкретные модули
RUST_LOG=bsl_backend::domain=debug cargo run --bin bsl-lsp-server

# Trace уровень (очень подробно)
RUST_LOG=trace cargo run --bin bsl-web-server -- --port 3002
```

### Проверка логов

```bash
# LSP Server логи
tail -f rust_lsp_server.log

# Web Server логи (stdout)
cargo run --bin bsl-web-server -- --port 3002 2>&1 | tee web_server.log
```

### Backtrace

```bash
# Включить backtrace при панике
RUST_BACKTRACE=1 cargo run --bin bsl-lsp-server

# Полный backtrace
RUST_BACKTRACE=full cargo test
```

---

## 🔄 Обновление зависимостей

### Проверка устаревших зависимостей

```bash
# Установка cargo-outdated (первый раз)
cargo install cargo-outdated

# Проверка устаревших зависимостей
cargo outdated
```

### Обновление

```bash
# Обновить все зависимости (по семантическому версионированию)
cargo update

# Обновить конкретную зависимость
cargo update -p serde

# Проверить, что всё компилируется
cargo check --workspace
cargo test
```

---

## 🧹 Очистка

### Очистка build артефактов

```bash
# Очистка target/
cargo clean

# Очистка конкретного крейта
cargo clean -p bsl-backend

# Очистка с сохранением зависимостей
cargo clean --release
```

### Очистка кеша

```bash
# Очистка .bsl_cache/
rm -rf .bsl_cache/

# Очистка node_modules (если нужно)
cd vscode-extension
rm -rf node_modules
npm install
cd ..
```

---

## 🎯 Полезные комбинации

### Быстрая разработка

```bash
# 1. Компиляция с оптимизацией разработки
cargo build --profile dev-fast

# 2. Запуск тестов
cargo test

# 3. Запуск LSP Server
cargo run --bin bsl-lsp-server
```

### Перед коммитом

```bash
# 1. Форматирование
cargo fmt

# 2. Линтинг
cargo clippy --workspace

# 3. Все тесты
cargo test --workspace

# 4. Проверка компиляции
cargo check --workspace --all-targets
```

### Перед релизом

```bash
# 1. Все тесты
cargo test --workspace

# 2. Integration тесты
cargo test --test '*'

# 3. Релизная сборка
cargo build --release

# 4. Упаковка Extension
cd vscode-extension
npm run compile
vsce package
cd ..
```

---

## ⚙️ Режимы сборки

### Profile: dev (по умолчанию)

```toml
[profile.dev]
opt-level = 0
debug = true
```

**Использование:** Быстрая компиляция, медленное выполнение

```bash
cargo build
```

### Profile: dev-fast

```toml
[profile.dev-fast]
inherits = "dev"
opt-level = 1
```

**Использование:** Баланс между скоростью компиляции и выполнения

```bash
cargo build --profile dev-fast
```

### Profile: release

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

**Использование:** Максимальная оптимизация, медленная компиляция

```bash
cargo build --release
```

---

## 🔗 Связанные руководства

- **[Roadmap Verification](roadmap-verification.md)** — проверка выполнения Milestone
- **[Tooling Guide](tooling-guide.md)** — MCP инструменты, ast-grep, sourcebot
- **[Web API Reference](../api/web-api-reference.md)** — детальное описание API

---

## 🤖 Автоматизация

Для автоматизации используй Claude Skills:

```bash
# Комплексное тестирование
/test-runner

# Тестирование Web API
/api-tester

# Проверка Roadmap
/roadmap-checker
```

**См. также:** `.claude/skills/` для деталей каждого навыка
