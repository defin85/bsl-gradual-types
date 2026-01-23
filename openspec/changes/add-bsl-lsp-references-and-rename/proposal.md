# Change: add-bsl-lsp-references-and-rename

## Why
Find References и Rename — базовые “IDE‑grade” функции IntelliSense. Сейчас LSP‑сервер не объявляет `references_provider`/`rename_provider` (см. `backend/src/bin/lsp_server/server/language_server.rs:106`), поэтому в IDE невозможно безопасно навигировать по использованию символов и выполнять рефакторинг переименования.

## What Changes
- Добавить `textDocument/references` (поиск ссылок).
- Добавить `textDocument/rename` + `prepareRename` (где поддерживается) для безопасного переименования.
- Зафиксировать ограничения (какие символы поддерживаются) и контракты в `openspec/specs/bsl-intellisense/spec.md` (delta).

## Impact
- Спецификация: `openspec/specs/bsl-intellisense/spec.md` (delta).
- Код: LSP server (новые handlers + символическая модель/индекс).
- Тесты: LSP‑интеграционные тесты на references/rename.
