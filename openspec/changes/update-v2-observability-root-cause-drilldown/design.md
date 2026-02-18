## Context
В проекте уже есть единый v2 orchestration facade/runtime и stage-level observability, общие для LSP/web/MCP. По последним профилям деградация латентности чаще проявляется в runtime queue/IR/parse_result, но текущих метрик недостаточно для быстрого root-cause анализа:
- operation-level вклад в `*_other` неочевиден;
- cancellation/skip причины частично видны и не унифицированы;
- saturation budget-ов runtime и эффективность singleflight видны неполно.

Дополнительно важно, чтобы улучшения производительности в shared runtime давали измеримый эффект и в `bsl-agent`.

## Goals / Non-Goals
- Goals:
  - Сузить поиск проблемных мест до комбинации `origin+operation+stage` без взрыва кардинальности.
  - Сохранить совместимость с существующими fixed-key метриками в период миграции.
  - Обеспечить единый observability контракт: dual-write представления не должны иметь независимую семантику.
  - Сделать saturation/singleflight поведение прозрачно наблюдаемым.
  - Зафиксировать, что `bsl-agent` использует тот же контракт и корректный CPU class для batch-нагрузки.
- Non-Goals:
  - Переход на внешний TSDB/OTel backend в этом change.
  - Добавление high-cardinality лейблов (файлы, пути, символы, URI, user-input).
  - Редизайн публичных MCP/LSP API форматов за пределами snapshot-метрик.

## Decisions
- Decision: Ввести drilldown-контракт с ограниченной кардинальностью.
  - Для stage метрик используются только фиксированные enum-измерения: `origin`, `operation`, `stage`, `outcome|reason`.
  - Поля со свободным вводом запрещены в metric key.

- Decision: Один канонический контракт и два представления (drilldown + compatibility projection).
  - Канонический слой задаётся фиксированными измерениями `origin/operation/stage/outcome|reason`.
  - Legacy fixed-key метрики вычисляются как deterministic projection из канонического слоя.
  - Drilldown и legacy MUST NOT эмититься как независимые контракты с разной семантикой.

- Decision: Сохранить additive dual-write rollout.
  - Существующие fixed-key метрики продолжают публиковаться как compatibility projection.
  - Новые drilldown метрики публикуются параллельно как primary representation.
  - Это исключает поломку текущих tests/dashboards и позволяет постепенную миграцию.

- Decision: Выделить saturation/singleflight наблюдаемость в отдельный обязательный слой.
  - Нужны явные метрики waiters/permits/queue depth и singleflight effectiveness (`leader/shared` по query kind, `key_unavailable`).
  - Это позволяет отделять CPU contention от проблем конкретных semantic стадий.

- Decision: Зафиксировать perf-sensitive поведение `bsl-agent` для batch инструментов.
  - Долгие batch semantic операции в MCP пути должны выполняться как background workload class.
  - Интерактивные инструменты остаются latency-priority.

## Alternatives Considered
- Option A: Оставить только текущие fixed keys и добавить 2-3 новых счетчика.
  - Плюсы: минимальная реализация.
  - Минусы: не решает root-cause drilldown для `*_other`; triage остается дорогим.

- Option B: Полноценные label-based Prometheus/OTel метрики прямо сейчас.
  - Плюсы: стандартная модель.
  - Минусы: больше миграции и риск избыточной сложности для текущего JSON snapshot pipeline.

- Chosen: Additive drilldown в текущем формате + bounded dimensions.
  - Дает нужную диагностическую точность и минимизирует риск/объем изменений.

## Risks / Trade-offs
- Риск: рост объема метрик и накладных расходов.
  - Mitigation: фиксированные enum-измерения, без high-cardinality; ограничить набор обязательных комбинаций.

- Риск: двойной контракт (legacy + drilldown) временно усложняет тесты.
  - Mitigation: единая mapping-таблица и инвариантные tests "каноника -> legacy projection" + parity tests для LSP/MCP.

- Риск: перенос batch-инструментов `bsl-agent` в background может поменять распределение latency.
  - Mitigation: добавить perf smoke с конкурентной нагрузкой и проверить отсутствие starvation интерактивных запросов.

## Implementation Plan (High Level)
1. Зафиксировать каноническую schema-модель и deterministic mapping в compatibility fixed keys.
2. Расширить observability API в `bsl-runtime` под канонический drilldown и saturation слой; legacy публиковать только через projection.
3. Привязать emission к shared facade/runtime так, чтобы один и тот же контракт автоматически покрывал LSP и MCP.
4. Обновить `bsl-agent` batch paths на background class и проверить parity метрик.
5. Зафиксировать quality checks: контрактные tests для projection/parity + perf smoke для смешанной нагрузки.
