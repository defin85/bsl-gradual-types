## 1. Symbol identity
- [x] Определить, какие “символы” поддерживаются для references/rename (минимум: процедуры/функции и локальные переменные).
- [x] Определить формат symbol id / способ резолвинга (по IR/semantic info), чтобы results были стабильны.

## 2. LSP: References
- [x] Объявить `references_provider` в `ServerCapabilities`.
- [x] Реализовать `textDocument/references`:
  - [x] Корректные ranges и dedup.
  - [x] Уважать `includeDeclaration`.
- [x] Тесты: references на фикстурах (локальная переменная, процедура/функция).

## 3. LSP: Rename
- [x] Объявить `rename_provider` в `ServerCapabilities`.
- [x] Реализовать `textDocument/rename`:
  - [x] Fail‑fast для неподдерживаемых случаев (например, dynamic access / строковые имена).
  - [x] Возвращать WorkspaceEdit с корректными TextEdit.
- [x] (Опционально) `textDocument/prepareRename` для ранней валидации.
- [x] Тесты: rename меняет все ссылки и не трогает “похожие” идентификаторы.

## 4. Spec
- [x] Обновить `openspec/changes/add-bsl-lsp-references-and-rename/specs/bsl-intellisense/spec.md`.

## 5. Validation
- [x] `openspec validate add-bsl-lsp-references-and-rename --strict --no-interactive`
