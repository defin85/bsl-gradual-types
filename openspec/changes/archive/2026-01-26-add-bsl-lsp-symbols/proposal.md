# Change: add-bsl-lsp-symbols

## Why
Для современного IntelliSense в IDE важны “структура документа” и навигация по символам: outline/breadcrumbs, поиск символов в workspace и быстрый переход к ним. Сейчас LSP‑сервер объявляет completion/hover/signatureHelp/definition, но не объявляет `documentSymbol`/`workspaceSymbol` (см. `backend/src/bin/lsp_server/server/language_server.rs:106`).

## What Changes
- Добавить поддержку `textDocument/documentSymbol` (структура файла BSL).
- Добавить поддержку `workspace/symbol` (поиск символов по workspace) как best‑effort поверх индекса/снапшотов.
- Зафиксировать контракт в `openspec/specs/bsl-intellisense/spec.md` (delta через change).

## Impact
- Спецификация: `openspec/specs/bsl-intellisense/spec.md` (delta).
- Код: `backend/src/bin/lsp_server/server/language_server.rs` + новые handlers.
- Тесты: unit/интеграционные тесты LSP на documentSymbol/workspaceSymbol.
