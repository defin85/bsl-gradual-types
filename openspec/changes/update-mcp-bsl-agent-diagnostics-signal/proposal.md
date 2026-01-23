# Change: update-mcp-bsl-agent-diagnostics-signal

## Why
`bsl-agent` используется как MCP‑инструмент для получения диагностики по BSL. На реальном прогоне диагностики обнаружились два класса проблем, которые ухудшают “signal/noise” и сбивают клиентов (в т.ч. LLM):

- Неоднозначность `scope` для `bsl_diagnostics_start`: в описаниях фигурирует `file`, но строковый `scope="file"` не принимается (`unknown scope: file`), при этом файл‑scope реально поддержан только в tagged форме `{ "kind":"file", "document": ... }`.
- Шумные ошибки “несуществующий метод/свойство” на динамических типах вида `Dynamic.*` (например, `Dynamic.Объект`), потому что:
  - проверка “Dynamic types” не покрывает семейство `Dynamic.*`;
  - для доступа к свойствам отсутствует симметричное “skip validation for Dynamic types”, которое уже есть для вызовов методов.

Дополнительно, “Unknown type access” сейчас почти всегда поднимается как `Error`, даже когда это следствие неполной инференции (а не реальной ошибки кода), что на крупных проектах быстро “забивает” полезную диагностику.

## What Changes
- Уточнить и закрепить контракт `scope` для `bsl_diagnostics_start`:
  - строковые значения: только `project|hot`;
  - `file` доступен в tagged форме (`{ kind: file, document: ... }`) и этот формат явно показан в `mcp_help` и `#[tool(description=...)]`.
- Снизить ложные “member does not exist” ошибки на динамике:
  - считать типы `Dynamic.*` dynamic-like;
  - пропускать проверку существования метода/свойства, если receiver имеет dynamic-like тип.
- Пересмотреть severity для “Unknown type access”:
  - `Error` сохраняется для случаев, где причина неизвестности — реальная ошибка (например, undeclared variable / type not found);
  - в остальных случаях — `Warning` (или suppression, если конфигурация не загружена), чтобы диагностика оставалась полезной.

## Impact
- Спецификация: `openspec/specs/mcp-bsl-agent/spec.md` (delta через change).
- Код:
  - `bsl-agent` (описания tool-ов и `mcp_help` для `bsl_diagnostics_start`)
  - `semantic-diagnostics` + `shared` (динамика/unknown severity правила)
- Тесты:
  - unit/интеграционные регрессии на `Dynamic.*` и на `scope=file` (tagged) для `bsl_diagnostics_start`

