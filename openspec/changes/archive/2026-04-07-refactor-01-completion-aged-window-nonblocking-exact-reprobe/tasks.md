## 1. Specification

- [x] Зафиксировать step-1 contract для aged non-member non-blocking re-probe в `bsl-intellisense-v2`.
- [x] Зафиксировать truthful timeline coverage requirement для blocking current-revision snapshot reacquisition.

## 2. Implementation

- [x] Убрать синхронный exact re-probe из aged non-member current-revision first-response path в LSP completion handler.
- [x] Если blocking current-revision snapshot reacquisition где-то остаётся, сделать её явным low-cardinality stage в authoritative timeline или убрать с critical path.
- [x] Сохранить bounded lightweight/no-IR fallback без stale semantic substitute другой revision.

## 3. Validation

- [x] Добавить regression coverage для incident-like aged invoked completion profile с bounded truthful first response.
- [x] Добавить gate на stage coverage, чтобы representative aged traces не оставляли seconds-scale uncovered handler gap.
- [x] Прогнать `openspec validate refactor-01-completion-aged-window-nonblocking-exact-reprobe --strict --no-interactive`.
