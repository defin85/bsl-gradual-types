# Автоматизированные навыки (Claude Skills)

Используй Skill tool для автоматизации частых задач.

## Доступные Skills

### /build
**Комплексная сборка всех компонентов проекта**

Что делает:
- Запускает `build-all.sh` скрипт
- Сборка Rust бинарников (LSP, Web, CLI)
- VSCode Extension
- Копирование в bin/
- Сборка WASM
- Проверка целостности

**Файл:** `.claude/skills/build.md`
**Скрипт:** `build-all.sh`

---

### /test-runner
**Комплексное тестирование проекта**

Что делает:
- Rust unit + integration тесты
- TypeScript тесты
- Compilation checks

**Файл:** `.claude/skills/test-runner.md`

---

### /api-tester
**Тестирование BSL Web API**

Что делает:
- Проверка всех endpoints
- URL-encoding для кириллицы

**Файл:** `.claude/skills/api-tester.md`

---

### /roadmap-checker
**Автоматическая проверка выполнения Milestone задач**

Что делает:
- grep/Read/cargo test для честной проверки прогресса

**Файл:** `.claude/skills/roadmap-checker.md`

---

### /web-ui
**Запуск веб-сервера с UI для просмотра типов**

Что делает:
- Сборка frontend (WASM через Trunk)
- Копирование статики
- Запуск веб-сервера с platform types
- Открытие браузера на http://127.0.0.1:8080

**Файл:** `.claude/skills/web-ui.md`

---

### /test-progress
**Тестирование прогресса парсинга (Windows)**

Что делает:
- Очистка Windows File System Cache
- Сборка LSP
- Копирование в расширение
- Инструкции для тестирования прогресса парсинга platform types

**Файл:** `.claude/skills/test-progress.md`
**Требования:** Windows 10/11, права администратора

---

### /start-lsp-api
**Запуск Web API для автоматизированного тестирования LSP**

Что делает:
- Сборка и запуск bsl-web-server
- Тестирование LSP функций через HTTP API
- POST /api/validate для semantic diagnostics
- GET /api/search для типов

Позволяет Claude автоматически тестировать исправления без VSCode.

**Файл:** `.claude/skills/start-lsp-api.md`
**Порт:** http://localhost:3002
