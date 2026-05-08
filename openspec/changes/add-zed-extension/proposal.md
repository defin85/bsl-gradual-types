# Change: Добавить расширение для Zed Editor

## Why
Zed Editor набирает популярность среди разработчиков 1С, использующих WSL/Linux. Сейчас инструментарий BSL Gradual Types доступен только через VS Code extension. Пользователи Zed вынуждены либо переключаться на VS Code для типового анализа, либо работать без него.

Вся ядровая логика (analysis-v2, LSP-сервер, диагностики) реализована на Rust и переиспользуется на 100%. VS Code extension при этом — это TypeScript-обёртка, специфичная для VS Code API, и непригодна для порта.

Zed поддерживает расширения на Rust/WebAssembly + возможность запуска внешнего LSP-сервера — это минимальный и естественный путь для интеграции.

## What Changes
- **Новый capability** `bsl-zed-extension`:
  - Расширение Zed, регистрирующее язык BSL с поддержкой Tree-sitter грамматики
  - Wasm-загрузчик, embed-ящий `bsl-lsp-server`, разворачивающий его в Zed extension work dir и запускающий как Language Server
  - Build-процесс, материализующий generated artifacts `bsl-lsp-server` и `extension.wasm` для dev-extension bundle (первичная цель — WSL/Linux)
  - Tree-sitter queries для подсветки синтаксиса (адаптированы из существующей грамматики)
- **Backend hardening для smoke-gate**:
  - `bsl-lsp-server` didSave save-fastlane предпочитает same-version ready parse snapshot перед applied-analysis fallback
  - Если ready snapshot ещё содержит deferred syntax-error assembly, save-fastlane достраивает syntax errors из `backend_tree` перед первой публикацией диагностик
- **Не затрагивает**: ядро анализа, VS Code extension, CLI, веб-сервер

## Impact
- Affected specs (новый): `bsl-zed-extension`
- Affected code (будет создан):
  - `zed-extension/` — корневая директория расширения
  - `zed-extension/extension.toml` — манифест
  - `zed-extension/Cargo.toml` — Rust crate для Wasm-загрузчика
  - `zed-extension/src/lib.rs` — Wasm-загрузчик LSP
  - `zed-extension/languages/bsl/` — Tree-sitter queries и конфигурация языка
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` — didSave save-fastlane ready-snapshot fallback
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` — shared ready-snapshot deferred syntax assembly updater

## Non-Goals
- Поддержка macOS и Windows native (только WSL/Linux на первом этапе)
- Портирование VS Code webview-панелей в Zed
- Регистрация DAP/MCP серверов в расширении
- Публикация в официальный extensions registry Zed (dev-extension достаточно)
- Широкие изменения в `analysis-v2` или публичном LSP-контракте
