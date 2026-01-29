## 1. Capabilities & gating
- [x] Добавить `inlay_hint_provider` / `code_action_provider` в `ServerCapabilities` только при включённых флагах и наличии реализации.
- [x] Зафиксировать поведение при выключенных фичах: сервер не заявляет capability (предпочтительно) или возвращает предсказуемый отказ.

## 2. Конфигурация
- [x] Определить, какие настройки приходят из:
  - `initializationOptions` (feature gate: enableTypeHints/enableCodeActions),
  - `workspace/didChangeConfiguration` (тонкие настройки: пороги/детализация).
- [x] Добавить/расширить структуры настроек для `bsl.typeHints.*` и `bsl.codeActions.*` в backend.

## 3. Inlay hints (MVP)
- [x] Реализовать `textDocument/inlayHint` (range-based) для `.bsl/.os`.
- [x] Минимальная полезность: hints типа `: <TypeName>` для локальных переменных (VarDeclaration); опционально — для return type функций.
- [x] Фильтрация по “шуму”: порог уверенности (`minCertainty`) и флаги включения категорий hints.
- [x] Ограничение результата (например, max 200 hints) и детерминизм порядка.

## 4. Code actions (MVP)
- [x] Реализовать `textDocument/codeAction`.
- [x] Минимальная полезность: хотя бы 1 refactor action и хотя бы 1 quick fix с детерминированной логикой (без regex по тексту diagnostics).
- [x] Поддержать отмену/таймауты (не блокировать сервер на долгих вычислениях).
- [x] Ограничить область применимости (MVP), задокументировать в spec.

## 5. Тесты
- [x] Тест на capability gating: включено/выключено → capability объявлен/не объявлен.
- [x] Тесты inlay hints: на простом BSL сниппете hints не пустые, позиции корректные, порядок детерминирован.
- [x] Тесты code actions: возвращается ожидаемый набор действий, WorkspaceEdit корректен и не трогает лишнее.

## 6. Spec delta + validate
- [x] Обновить `openspec/changes/add-bsl-lsp-inlay-hints-and-code-actions/specs/bsl-intellisense/spec.md`.
- [x] `openspec validate add-bsl-lsp-inlay-hints-and-code-actions --strict --no-interactive`
