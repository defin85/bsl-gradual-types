## 1. Symbol identity
- [ ] Определить, какие “символы” поддерживаются для references/rename (минимум: процедуры/функции и локальные переменные).
- [ ] Определить формат symbol id / способ резолвинга (по IR/semantic info), чтобы results были стабильны.

## 2. LSP: References
- [ ] Объявить `references_provider` в `ServerCapabilities`.
- [ ] Реализовать `textDocument/references`:
  - [ ] Корректные ranges и dedup.
  - [ ] Уважать `includeDeclaration`.
- [ ] Тесты: references на фикстурах (локальная переменная, процедура/функция).

## 3. LSP: Rename
- [ ] Объявить `rename_provider` в `ServerCapabilities`.
- [ ] Реализовать `textDocument/rename`:
  - [ ] Fail‑fast для неподдерживаемых случаев (например, dynamic access / строковые имена).
  - [ ] Возвращать WorkspaceEdit с корректными TextEdit.
- [ ] (Опционально) `textDocument/prepareRename` для ранней валидации.
- [ ] Тесты: rename меняет все ссылки и не трогает “похожие” идентификаторы.

## 4. Spec
- [ ] Обновить `openspec/changes/add-bsl-lsp-references-and-rename/specs/bsl-intellisense/spec.md`.

## 5. Validation
- [ ] `openspec validate add-bsl-lsp-references-and-rename --strict --no-interactive`
