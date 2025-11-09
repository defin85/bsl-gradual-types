# CLAUDE.md

AI-ассистент инструкции для BSL Gradual Type System проекта.

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
Что делает: сборка Rust бинарников (LSP, Web, CLI), VSCode Extension, копирование в bin/
**Файл:** [.claude/skills/build.md](.claude/skills/build.md)

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
