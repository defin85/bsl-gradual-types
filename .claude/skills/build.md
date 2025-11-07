# Build Skill

Комплексная сборка всех компонентов BSL Gradual Types проекта.

## 🎯 Назначение

Автоматизированная сборка всех бинарников и компонентов проекта:
- 🦀 Rust бинарники (LSP, Web Server, CLI)
- 📦 VSCode Extension
- 🔄 Копирование бинарников в расширение
- ✅ Проверка целостности сборки

## 🔧 Процесс сборки

### 1. Rust бинарники (Release mode)

```bash
# LSP Server
cargo build --release -p bsl-backend --bin bsl-lsp-server

# Web Server
cargo build --release -p bsl-backend --bin bsl-web-server

# CLI
cargo build --release -p bsl-cli
```

**Результат:**
- `target/release/bsl-lsp-server.exe` (~9 MB)
- `target/release/bsl-web-server.exe` (~14 MB)
- `target/release/bsl-cli.exe` (~3 MB)

### 2. VSCode Extension

```bash
cd vscode-extension

# Компиляция TypeScript + копирование бинарников + сборка webview
npm run compile

cd ..
```

**Что происходит:**
1. `npm run copy-binaries` - копирует Rust бинарники из `target/release/` в `vscode-extension/bin/`
2. `node esbuild.js` - компилирует TypeScript код расширения
3. `npm run build:webview` - собирает React webview с Tailwind CSS

**Результат:**
- `vscode-extension/out/extension.js` - код расширения
- `vscode-extension/bin/lsp_server.exe` - LSP Server для расширения
- `vscode-extension/media/webview/*.js|css` - webview assets

### 3. Проверка целостности

```bash
# Проверка существования бинарников
ls -lh target/release/bsl-*.exe
ls -lh vscode-extension/bin/lsp_server.exe

# Проверка расширения
ls -lh vscode-extension/out/extension.js
ls -lh vscode-extension/media/webview/
```

## 📊 Формат отчёта

```markdown
# 🏗️ Отчёт о сборке BSL Gradual Types

**Дата:** 2025-11-05
**Версия:** 0.4.0

---

## ✅ Rust бинарники (Release)

**Команда:** `cargo build --release -p bsl-backend --bin bsl-lsp-server`

**Результат:**
- ✅ bsl-lsp-server: сборка успешна (9.09 MB)
- ✅ bsl-web-server: сборка успешна (14.23 MB)
- ✅ bsl-cli: сборка успешна (2.87 MB)

**Время сборки:** 2м 30с

---

## ✅ VSCode Extension

**Команда:** `cd vscode-extension && npm run compile`

**Результат:**
- ✅ TypeScript compilation: успешна
- ✅ Webview build (Vite + Tailwind): успешна
- ✅ Бинарники скопированы в vscode-extension/bin/

**Файлы:**
- `out/extension.js` (245 KB)
- `bin/lsp_server.exe` (9.09 MB)
- `media/webview/tailwind.js` (141 KB)
- `media/webview/typeDetails.js` (6 KB)

**Время сборки:** 15 секунд

---

## ✅ Проверка целостности

**Все необходимые файлы на месте:**
- ✅ target/release/bsl-lsp-server.exe
- ✅ target/release/bsl-web-server.exe
- ✅ target/release/bsl-cli.exe
- ✅ vscode-extension/bin/lsp_server.exe
- ✅ vscode-extension/out/extension.js

---

## 📊 Общий итог

| Компонент | Результат | Размер | Статус |
|-----------|-----------|--------|--------|
| LSP Server | Собран | 9.09 MB | ✅ |
| Web Server | Собран | 14.23 MB | ✅ |
| CLI | Собран | 2.87 MB | ✅ |
| VSCode Extension | Собран | 245 KB | ✅ |
| Webview Assets | Собраны | 148 KB | ✅ |

**Общая оценка:** ✅ **Сборка успешна**

**Время выполнения:** 2м 45с
**Следующий шаг:** Тестирование (`/test-runner`)

---
```

## ❌ Обработка ошибок сборки

### Если cargo build провалился

```markdown
## ❌ Rust бинарники

**Команда:** `cargo build --release -p bsl-backend --bin bsl-lsp-server`

**Результат:**
- ❌ Ошибка компиляции

**Ошибка:**
```
error[E0425]: cannot find function `resolve_type` in this scope
  --> backend/src/domain/type_resolver.rs:145:20
   |
145 |         let ty = resolve_type(&context)?;
   |                  ^^^^^^^^^^^^ not found in this scope
```

**Файл:** `backend/src/domain/type_resolver.rs:145`

**Причина:** Отсутствует импорт функции `resolve_type`

**Рекомендация:** Добавить `use crate::domain::resolver::resolve_type;`

---

## 🚨 Критический провал: Исправить немедленно!

**Не запускать расширение** до исправления ошибок сборки.
```

### Если npm run compile провалился

```markdown
## ❌ VSCode Extension

**Команда:** `cd vscode-extension && npm run compile`

**Результат:**
- ❌ TypeScript compilation failed

**Ошибка:**
```
src/extension.ts(45,12): error TS2345: Argument of type 'string' is not assignable to parameter of type 'Uri'.
```

**Файл:** `vscode-extension/src/extension.ts:45`

**Причина:** Неверный тип аргумента для Uri

**Рекомендация:** Использовать `Uri.file(path)` вместо `path`

---
```

## 🎯 Использование

Запусти этот навык когда:
- После изменений в Rust коде
- После изменений в TypeScript коде расширения
- Перед тестированием (`/test-runner`)
- Перед созданием VSIX package
- После обновления зависимостей

**Команда:**
```
/build
```

**Или:**
```
Собери все компоненты проекта
```

## ⚙️ Опции сборки

### Быстрая сборка (Debug mode)

```bash
# Для разработки (быстрее, но больше размер)
cargo build -p bsl-backend --bin bsl-lsp-server

# Время: ~30 секунд
# Размер: ~20 MB
```

### Полная сборка (Release mode с оптимизациями)

```bash
# Для production (медленнее, но меньше размер)
cargo build --release -p bsl-backend --bin bsl-lsp-server

# Время: ~2-3 минуты
# Размер: ~9 MB
```

### Сборка с проверками

```bash
# Сборка + clippy + форматирование
cargo clippy --release -p bsl-backend --bin bsl-lsp-server
cargo fmt --check

# Полезно перед коммитом
```

## 🔄 Автоматическая сборка при изменениях

### Rust (cargo watch)

```bash
# Установить cargo-watch
cargo install cargo-watch

# Автоматическая пересборка при изменениях
cargo watch -x "build --release -p bsl-backend --bin bsl-lsp-server"
```

### TypeScript (npm watch)

```bash
cd vscode-extension

# Автоматическая компиляция при изменениях
npm run watch

cd ..
```

## 📦 Упаковка расширения (VSIX)

```bash
cd vscode-extension

# Собрать и упаковать расширение
vsce package

# Результат: bsl-gradual-types-1.0.0.vsix

cd ..
```

**Требования:**
- Все компоненты собраны (`/build`)
- Все тесты прошли (`/test-runner`)
- Нет ошибок линтинга

## ⚠️ Особенности проекта

### GitBash на Windows

Все команды используют Unix-style синтаксис:

```bash
# ✅ Работает в GitBash
cargo build --release -p bsl-backend --bin bsl-lsp-server

# ❌ НЕ работает (PowerShell syntax)
cargo build /release /p bsl-backend
```

### Копирование бинарников

**Автоматическое:**
- `npm run compile` в vscode-extension автоматически вызывает `npm run copy-binaries`
- Скрипт `scripts/copy-binaries.js` копирует Rust бинарники

**Ручное:**
```bash
# Если нужно скопировать вручную
cp target/release/bsl-lsp-server.exe vscode-extension/bin/lsp_server.exe
```

### Зависимости между компонентами

```
1. Rust бинарники (cargo build)
       ↓
2. Копирование бинарников (npm run copy-binaries)
       ↓
3. TypeScript компиляция (npm run compile)
       ↓
4. Webview сборка (npm run build:webview)
       ↓
5. Упаковка VSIX (vsce package)
```

**Важно:** Не пропускай шаги! Каждый зависит от предыдущего.

## 🔗 Связанные навыки

- **test-runner** - тестирование после сборки
- **api-tester** - проверка Web API
- **roadmap-checker** - проверка выполнения Milestone задач

## 📚 Документация

- **[docs/guides/development-workflow.md](../../docs/guides/development-workflow.md)** - детальные команды разработки
- **[vscode-extension/README.md](../../vscode-extension/README.md)** - сборка расширения
- **[vscode-extension/scripts/copy-binaries.js](../../vscode-extension/scripts/copy-binaries.js)** - скрипт копирования
