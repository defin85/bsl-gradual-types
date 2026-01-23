## 1. Implementation
- [x] Зафиксировать контракт `scope` для `bsl_diagnostics_start`:
  - [x] `scope="project"|"hot"` — поддержаны как строки
  - [x] `scope.kind="file"` — поддержан как tagged (с обязательным `document`)
  - [x] При `scope="file"` (строка) — ошибка `INVALID_PARAMS` с подсказкой “use tagged file scope”
- [x] Обновить `mcp_help` и `#[tool(description=...)]` для `bsl_diagnostics_start`:
  - [x] Добавить пример вызова с `scope: { kind: "file", document: { path: "/abs/..." } }`
  - [x] Убрать двусмысленность “scope: project|hot|file” в пользу точной формулировки (1 строка)
- [x] Снизить шум по `Dynamic.*`:
  - [x] Расширить “dynamic-like” детекцию (включая `Dynamic.<facet>`), чтобы она работала одинаково для method/property access
  - [x] Пропускать validate_method_exists/validate_property_exists для dynamic-like receiver’ов
- [x] Пересмотреть severity для unknown member access:
  - [x] `UndeclaredVariable`/`TypeNotFound` → `Error`
  - [x] `ConfigurationNotLoaded` → suppression (graceful degradation)
  - [x] прочие unknown причины → `Warning`

## 2. Tests
- [x] `bsl-agent` stdio integration:
  - [x] `bsl_diagnostics_start` с tagged file scope не требует `workspace_documents_set(mark_hot=true)`
  - [x] Ошибка для `scope="file"` (строка) содержит подсказку по формату
- [x] `semantic-diagnostics` unit tests:
  - [x] Property access на `Dynamic.Объект` не генерирует “NonExistentProperty”
  - [x] Method call на `Dynamic.Объект` не генерирует “NonExistentMethod”
  - [x] UnknownTypeAccess severity деградирует до Warning, если причина неизвестности не “undeclared”

## 3. Docs
- [x] Обновить `bsl-agent/README.md` (коротко, рядом с примечаниями про `scope`)
