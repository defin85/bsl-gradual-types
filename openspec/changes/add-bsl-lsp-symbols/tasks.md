## 1. LSP: Document Symbols
- [x] Объявить `document_symbol_provider` в `ServerCapabilities`.
- [x] Реализовать `textDocument/documentSymbol`:
  - [x] Минимальный набор: процедуры/функции, области (`#Область`), экспортируемые элементы (если различимо).
  - [x] Корректные ranges (UTF‑16 позиции) и стабильность результата при одинаковом тексте.
- [x] Тесты: `documentSymbol` возвращает ожидаемую структуру на фикстурах.

## 2. LSP: Workspace Symbols
- [x] Объявить `workspace_symbol_provider` в `ServerCapabilities`.
- [x] Реализовать `workspace/symbol` (поиск по запросу):
  - [x] Минимально: по открытым/проиндексированным документам (с понятным поведением).
  - [x] Ограничения/качество выдачи зафиксировать в `design.md`.
- [x] Тесты: запросы по workspaceSymbol возвращают ожидаемые элементы на маленьком workspace.

## 3. Spec
- [x] Обновить `openspec/changes/add-bsl-lsp-symbols/specs/bsl-intellisense/spec.md`.

## 4. Validation
- [x] `openspec validate add-bsl-lsp-symbols --strict --no-interactive`
