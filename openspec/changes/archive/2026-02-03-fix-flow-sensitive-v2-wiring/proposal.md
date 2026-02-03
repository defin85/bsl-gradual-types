# Change: Исправление и доведение flow-sensitive v2 wiring (CFG + интерфейсы)

## Why
Сейчас в кодовой базе присутствуют элементы flow-sensitive анализа (CFG в IR, доменные анализаторы narrowing/null-safety, частичные интеграции),
но итоговое поведение не соответствует ожидаемому контракту:
- flow-sensitive результаты не доступны/не согласованы между IDE (LSP), Web API и MCP;
- нет надёжного feature gate (default OFF) — в ряде мест flow-sensitive вычисления включаются “молча” или неуправляемо;
- CFG не гарантированно присутствует в v2 snapshot, а привязка “позиция → CFG контекст” опирается на эвристики;
- Web API/MCP контракт по флагам расходится с проектными ожиданиями (имена полей, дефолты, явная сигнализация включённости).

Это делает механизм фактически неработоспособным и создаёт риск дрейфа: разные интерфейсы начинают “чинить” flow-sensitive локальными эвристиками.

## What Changes
- Зафиксировать и реализовать единый контракт flow-sensitive режима в v2:
  - включение только по явному флагу/настройке (default: OFF),
  - единая семантика для IDE/LSP, Web API и MCP.
- Стабилизировать технический фундамент:
  - `SemanticProgram.cfg` всегда присутствует (минимум: `Entry -> Exit`),
  - детерминированный и bias-aware выбор CFG узла по byte offset (единый API).
- Встроить flow-sensitive как v2-only вычисления (без legacy inference путей):
  - type-at-position с учётом narrowing,
  - null-safety diagnostics, добавляемые только при включённом режиме.
- Исправить wiring в интерфейсах:
  - LSP: workspace setting `enableFlowSensitive` (default false) + использование flow-sensitive результатов при включении,
  - Web API: единый параметр `includeFlowSensitive` (default false) и явное отклонение legacy `include_flow_sensitive`,
  - MCP: `include_flow_sensitive` (default false) + явное поле/флаг в ответах, что режим включён.

## Impact
- Affected specs:
  - `bsl-intellisense-v2` (CFG контракт, position→CFG mapping, flow-sensitive gating и поведение)
  - `mcp-bsl-agent` (параметры и выходы инструментов для flow-sensitive режима)
- Affected code (после утверждения и реализации):
  - `shared/` (CFG API + доменные анализаторы)
  - `analysis-v2/` (flow-sensitive v2 queries / gating)
  - `backend/` (LSP + Web API)
  - `bsl-agent/` (MCP tools)

## Supersedes
Этот change заменяет и закрывает как “superseded” следующие change’и:
- `integrate-flow-sensitive-v2-wiring`
- `update-flow-sensitive-cfg-stability`
- `refactor-flow-sensitive-cfg`

## Non-Goals
- Делать flow-sensitive режим включённым по умолчанию.
- Делать “идеальный” анализ для всех конструкций языка; фокус на корректном контракте и воспроизводимости.
- Вводить альтернативный pipeline вне `bsl-analysis-v2`/deps snapshot.

