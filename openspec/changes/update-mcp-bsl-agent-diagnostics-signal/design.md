# Design: improve diagnostics signal for MCP `bsl-agent`

## 1) `scope` контракт для `bsl_diagnostics_start`
Проблема: в tool‑описаниях фигурирует `file`, но строковый `scope="file"` невалиден и приводит к `unknown scope: file`. Tagged `WorkspaceScopeTagged::File { document }` уже существует и работает, но это не очевидно клиенту.

Решение:
- Строковые `scope` остаются LLM-friendly, но только для простых вариантов: `project|hot`.
- Для file‑scope обязателен tagged формат: `{ "kind":"file", "document": <DocumentRef> }`.
- Если клиент прислал строку `scope="file"`, сервер возвращает `INVALID_PARAMS` с подсказкой корректного формата.
- `mcp_help(tool_name="bsl_diagnostics_start")` возвращает минимум 1 пример с tagged file scope.
- `#[tool(description=...)]` уточняет scope в 1 строку, без двусмысленности.

## 2) Dynamic-like типы (`Dynamic.*`)
Проблема: в практике инференса встречаются platform‑типы с именами `Dynamic.<Facet>` (например, `Dynamic.Объект`). Они не являются `ResolutionResult::Dynamic`, поэтому текущая проверка `TypeResolution::is_dynamic()` их не распознаёт. В результате валидатор пытается проверять методы/свойства и генерирует малоинформативные ошибки.

Решение:
- Ввести единое понятие “dynamic-like”:
  - `ResolutionResult::Dynamic`
  - platform type, имя которого равно `Dynamic` или начинается с `Dynamic.`
- Для `FunctionCall` и `MemberAccess(Property)` использовать одинаковую политику:
  - если receiver dynamic-like → пропустить проверку существования члена (ошибка уже должна проявиться ранее в цепочке, либо это реально динамика)

## 3) Unknown member access severity
Проблема: “Unknown type access” (не удалось вывести тип) часто является следствием ограничений инференса и засоряет список ошибок.

Решение:
- Severity зависит от `uncertainty_reason`:
  - `ConfigurationNotLoaded` → suppression (как graceful degradation)
  - `UndeclaredVariable`/`TypeNotFound` → `Error`
  - прочие unknown → `Warning`

Дополнение:
- При необходимости можно расширить MCP DTO (`DiagnosticDto.code`) в будущем, но в рамках этого change достаточно выровнять severity и устранить динамический шум.

