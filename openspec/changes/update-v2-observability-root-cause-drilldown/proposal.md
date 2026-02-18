# Change: Update v2 observability root-cause drilldown

## Why
Текущий snapshot метрик позволяет увидеть общий рост latency, но плохо локализует первопричину:
- значимая часть нагрузки попадает в агрегированные корзины `*_other`;
- сложно отделить вклад operation/stage/cancellation/queue saturation;
- `bsl-agent` использует shared v2 facade/runtime, но для batch-сценариев не зафиксировано требование, которое гарантирует влияние perf-улучшений на MCP путь.

В результате triage и регрессии по производительности занимают больше времени, чем должны.

## What Changes
- Добавить в `bsl-intellisense-v2` единый канонический root-cause контракт метрик с низкой кардинальностью: `origin + operation + stage + outcome/cause`.
- Зафиксировать канонический event model как единственный source of truth для observability emission:
  - событие несёт нормализованные измерения (enum-only, low-cardinality);
  - event -> metrics трансформация выполняется в одном backend-first projection слое.
- Зафиксировать backward-compatible rollout как dual-write **из одного канонического источника событий**:
  - drilldown метрики — основное представление контракта;
  - legacy fixed keys — совместимая проекция того же контракта без отдельной семантики.
- Зафиксировать ownership emission path:
  - адаптеры (LSP/web/MCP) эмитят только канонические события;
  - прямой emission drilldown/legacy метрик из адаптеров не допускается.
- Добавить требования к saturation/singleflight observability, чтобы явно видеть узкие места очередей и дедупликации.
- Зафиксировать в `mcp-bsl-agent` обязательное применение того же канонического контракта (включая dual-write-проекцию) и background-класса для долгих batch semantic инструментов.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `mcp-bsl-agent`
- Affected code (implementation stage):
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/system_coordinator/coordinator.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-agent/src/session/mod.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
