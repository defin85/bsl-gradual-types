# Build Skill

Комплексная сборка всех компонентов BSL Gradual Types проекта.

## 🎯 Назначение

Автоматизированная сборка всех бинарников и компонентов проекта:
- 🦀 Rust бинарники (LSP, Web Server, CLI)
- 📦 VSCode Extension
- 🔄 Копирование бинарников в расширение
- ✅ Проверка целостности сборки

## 🚀 Главный скрипт сборки

### Универсальный скрипт (РЕКОМЕНДУЕТСЯ)

```bash
# Полная сборка (Release mode)
./build-all.sh

# Быстрая сборка (Debug mode)
./build-all.sh --debug

# Без тестов (быстрее)
./build-all.sh --skip-tests
```

**Что делает `build-all.sh`:**
1. ✅ Собирает все Rust бинарники (LSP, Web, CLI)
2. ✅ Копирует бинарники в `vscode-extension/bin/`
3. ✅ Компилирует TypeScript код расширения
4. ✅ Собирает WASM bundles для webview
5. ✅ Запускает quick tests (опционально)
6. ✅ Показывает итоговый отчёт

**Время выполнения:**
- Release mode: ~3-4 минуты
- Debug mode: ~30-40 секунд

---

## 🔧 Ручной процесс сборки (если нужен)

### 1. Rust бинарники (Release mode)

```bash
# Все бинарники сразу
cargo build --release --workspace

# Или по отдельности:
cargo build --release -p bsl-backend --bin bsl-lsp-server
cargo build --release -p bsl-backend --bin bsl-web-server
cargo build --release -p bsl-cli
```

**Результат:**
- `target/release/bsl-lsp-server.exe` (~9-10 MB)
- `target/release/bsl-web-server.exe` (~14 MB)
- `target/release/bsl-cli.exe` (~3 MB)

### 2. VSCode Extension

```bash
cd vscode-extension

# Компиляция TypeScript + копирование бинарников + сборка WASM
npm run compile

cd ..
```

**Что происходит:**
1. `npm run copy-binaries` - копирует Rust бинарники из `target/release/` в `vscode-extension/bin/`
2. `node esbuild.js` - компилирует TypeScript код расширения
3. `npm run build:wasm:release` - собирает WASM bundles для webview

**Результат:**
- `vscode-extension/out/extension.js` - код расширения
- `vscode-extension/bin/lsp-server.exe` - LSP Server для расширения
- `vscode-extension/media/webview/*.wasm` - WASM assets

### 3. Проверка целостности

```bash
# Проверка существования бинарников
ls -lh target/release/bsl-*.exe
ls -lh vscode-extension/bin/lsp-server.exe

# Проверка расширения
ls -lh vscode-extension/out/extension.js
ls -lh vscode-extension/media/webview/
```

## 📊 Пример вывода `build-all.sh`

```
============================================================
🏗️  BSL Gradual Types - Полная сборка
============================================================

Режим сборки: release
Тесты: включены

============================================================
ЭТАП 1: Сборка Rust бинарников (release)
============================================================

🦀 Компиляция Rust проекта...
Режим: release
   Compiling bsl-shared v0.4.2
   Compiling bsl-backend v0.4.2
   Compiling bsl-cli v0.4.2
    Finished `release` profile [optimized] target(s) in 2m 15s

⏱️  Время выполнения: 135s

✅ Rust бинарники собраны

📦 Проверка собранных бинарников:
  ✅ LSP Server (9.8M)
  ✅ Web Server (14.2M)
  ✅ CLI (3.1M)

============================================================
ЭТАП 2: Копирование бинарников в VSCode Extension
============================================================

📋 Копирование бинарников:
Источник: target/release/
Назначение: vscode-extension/bin/

🔍 LSP Server:
  ✅ Скопирован lsp-server.exe (9.8M)

✅ Бинарники скопированы успешно

============================================================
ЭТАП 3: Сборка VSCode Extension
============================================================

📦 Установка зависимостей (если нужно)...
  ⏭️  node_modules существует, пропускаем npm install

🔨 Компиляция TypeScript + сборка WASM...
   ✅ TypeScript compiled
   ✅ WASM bundles built
⏱️  Время выполнения: 25s

📦 Проверка собранного расширения:
  ✅ Extension main file (24K)
  ✅ LSP Server binary (9.8M)

✅ VSCode Extension собрано успешно

============================================================
ЭТАП 4: Быстрые проверки
============================================================

🧪 Запуск быстрых unit тестов...
running 335 tests
test result: ok. 335 passed
⏱️  Время выполнения: 12s

✅ Быстрые тесты пройдены

============================================================
📊 ИТОГОВЫЙ ОТЧЁТ
============================================================

📦 Собранные компоненты:

🦀 Rust (release):
  ✅ LSP Server (9.8M)
  ✅ Web Server (14.2M)
  ✅ CLI (3.1M)

📦 VSCode Extension:
  ✅ TypeScript (24K)
  ✅ LSP Server binary (9.8M)
  ✅ WASM bundles (2 files)

✅ Все компоненты собраны успешно!

🚀 Следующие шаги:
  1. Запустить тесты: ./test-runner.sh или /test-runner
  2. Запустить VSCode: code vscode-extension/
  3. Проверить расширение: F5 в VSCode

⏱️  Общее время сборки: 172s

🎉 Сборка завершена успешно!
```

## 🎯 Использование

### Через Claude Skill (РЕКОМЕНДУЕТСЯ)

```
/build
```

**Что произойдёт:**
Claude автоматически запустит `./build-all.sh` и предоставит детальный отчёт о сборке.

### Через скрипт напрямую

```bash
# Полная сборка (release + тесты)
./build-all.sh

# Быстрая сборка (debug, без тестов)
./build-all.sh --debug --skip-tests
```

### Когда запускать сборку:

- ✅ После изменений в Rust коде
- ✅ После изменений в TypeScript коде расширения
- ✅ Перед тестированием (`/test-runner`)
- ✅ Перед созданием VSIX package
- ✅ После обновления зависимостей
- ✅ После git pull (если изменились бинарники)

## ⚙️ Опции скрипта build-all.sh

### Режимы сборки

| Опция | Описание | Время | Размер бинарника |
|-------|----------|-------|------------------|
| `./build-all.sh` | Release + тесты | ~3-4 мин | 9.8 MB |
| `./build-all.sh --debug` | Debug (быстро) | ~30-40 сек | 20 MB |
| `./build-all.sh --skip-tests` | Release без тестов | ~3 мин | 9.8 MB |
| `./build-all.sh --debug --skip-tests` | Самый быстрый | ~30 сек | 20 MB |

### Расширенная сборка с проверками

```bash
# Сборка + clippy + форматирование (перед коммитом)
cargo clippy --release --workspace
cargo fmt --check
./build-all.sh
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
