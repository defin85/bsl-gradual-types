## 1. Specification

- [x] Зафиксировать step-2 latest-only contract для VS Code current-context surface в `bsl-intellisense`.
- [x] Зафиксировать server-side supersession/coalescing contract для `bsl.getCurrentContext` в `bsl-intellisense-v2`.

## 2. Implementation

- [x] Пронести bounded editor-session/generation hints через `bsl.getCurrentContext` request path.
- [x] Сделать extension latest-only на apply path и игнорировать stale current-context responses.
- [x] Добавить backend supersession/coalescing для obsolete current-context work до expensive parse/context derivation.

## 3. Validation

- [x] Добавить cursor-burst regression coverage с bounded inflight/background current-context work и newest-generation-wins UI behavior.
- [x] Убедиться, что mixed-load profile не возвращает stale current-context surface после более нового cursor move.
- [x] Прогнать `openspec validate update-02-current-context-latest-only-supersession --strict --no-interactive`.
