## 1. LSP: Document Symbols
- [ ] Объявить `document_symbol_provider` в `ServerCapabilities`.
- [ ] Реализовать `textDocument/documentSymbol`:
  - [ ] Минимальный набор: процедуры/функции, области (`#Область`), экспортируемые элементы (если различимо).
  - [ ] Корректные ranges (UTF‑16 позиции) и стабильность результата при одинаковом тексте.
- [ ] Тесты: `documentSymbol` возвращает ожидаемую структуру на фикстурах.

## 2. LSP: Workspace Symbols
- [ ] Объявить `workspace_symbol_provider` в `ServerCapabilities`.
- [ ] Реализовать `workspace/symbol` (поиск по запросу):
  - [ ] Минимально: по открытым/проиндексированным документам (с понятным поведением).
  - [ ] Ограничения/качество выдачи зафиксировать в `design.md`.
- [ ] Тесты: запросы по workspaceSymbol возвращают ожидаемые элементы на маленьком workspace.

## 3. Spec
- [ ] Обновить `openspec/changes/add-bsl-lsp-symbols/specs/bsl-intellisense/spec.md`.

## 4. Validation
- [ ] `openspec validate add-bsl-lsp-symbols --strict --no-interactive`
