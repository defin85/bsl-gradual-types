## 1. Implementation
- [ ] Зафиксировать контракт `scope` для `bsl_diagnostics_start`:
  - [ ] `scope="project"|"hot"` — поддержаны как строки
  - [ ] `scope.kind="file"` — поддержан как tagged (с обязательным `document`)
  - [ ] При `scope="file"` (строка) — ошибка `INVALID_PARAMS` с подсказкой “use tagged file scope”
- [ ] Обновить `mcp_help` и `#[tool(description=...)]` для `bsl_diagnostics_start`:
  - [ ] Добавить пример вызова с `scope: { kind: "file", document: { path: "/abs/..." } }`
  - [ ] Убрать двусмысленность “scope: project|hot|file” в пользу точной формулировки (1 строка)
- [ ] Снизить шум по `Dynamic.*`:
  - [ ] Расширить “dynamic-like” детекцию (включая `Dynamic.<facet>`), чтобы она работала одинаково для method/property access
  - [ ] Пропускать validate_method_exists/validate_property_exists для dynamic-like receiver’ов
- [ ] Пересмотреть severity для unknown member access:
  - [ ] `UndeclaredVariable`/`TypeNotFound` → `Error`
  - [ ] `ConfigurationNotLoaded` → suppression (graceful degradation)
  - [ ] прочие unknown причины → `Warning`

## 2. Tests
- [ ] `bsl-agent` stdio integration:
  - [ ] `bsl_diagnostics_start` с tagged file scope не требует `workspace_documents_set(mark_hot=true)`
  - [ ] Ошибка для `scope="file"` (строка) содержит подсказку по формату
- [ ] `semantic-diagnostics` unit tests:
  - [ ] Property access на `Dynamic.Объект` не генерирует “NonExistentProperty”
  - [ ] Method call на `Dynamic.Объект` не генерирует “NonExistentMethod”
  - [ ] UnknownTypeAccess severity деградирует до Warning, если причина неизвестности не “undeclared”

## 3. Docs
- [ ] Обновить `bsl-agent/README.md` (коротко, рядом с примечаниями про `scope`)

