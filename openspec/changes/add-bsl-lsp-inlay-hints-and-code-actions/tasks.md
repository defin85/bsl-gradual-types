## 1. Capabilities & gating
- [ ] Добавить `inlay_hint_provider` / `code_action_provider` в `ServerCapabilities` только при включённых флагах и наличии реализации.
- [ ] Зафиксировать поведение при выключенных фичах: сервер не заявляет capability (предпочтительно) или возвращает предсказуемый отказ.

## 2. Конфигурация
- [ ] Определить, какие настройки приходят из:
  - `initializationOptions` (feature gate: enableTypeHints/enableCodeActions),
  - `workspace/didChangeConfiguration` (тонкие настройки: пороги/детализация).
- [ ] Добавить/расширить структуры настроек для `bsl.typeHints.*` и `bsl.codeActions.*` в backend.

## 3. Inlay hints (MVP)
- [ ] Реализовать `textDocument/inlayHint` (range-based) для `.bsl/.os`.
- [ ] Минимальная полезность: hints типа `: <TypeName>` для локальных переменных (VarDeclaration); опционально — для return type функций.
- [ ] Фильтрация по “шуму”: порог уверенности (`minCertainty`) и флаги включения категорий hints.
- [ ] Ограничение результата (например, max 200 hints) и детерминизм порядка.

## 4. Code actions (MVP)
- [ ] Реализовать `textDocument/codeAction`.
- [ ] Минимальная полезность: хотя бы 1 refactor action и хотя бы 1 quick fix с детерминированной логикой (без regex по тексту diagnostics).
- [ ] Поддержать отмену/таймауты (не блокировать сервер на долгих вычислениях).
- [ ] Ограничить область применимости (MVP), задокументировать в spec.

## 5. Тесты
- [ ] Тест на capability gating: включено/выключено → capability объявлен/не объявлен.
- [ ] Тесты inlay hints: на простом BSL сниппете hints не пустые, позиции корректные, порядок детерминирован.
- [ ] Тесты code actions: возвращается ожидаемый набор действий, WorkspaceEdit корректен и не трогает лишнее.

## 6. Spec delta + validate
- [ ] Обновить `openspec/changes/add-bsl-lsp-inlay-hints-and-code-actions/specs/bsl-intellisense/spec.md`.
- [ ] `openspec validate add-bsl-lsp-inlay-hints-and-code-actions --strict --no-interactive`

