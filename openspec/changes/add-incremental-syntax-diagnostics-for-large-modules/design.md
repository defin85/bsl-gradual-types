## Context
Инкрементальный `ParseSnapshot` уже существует как version-bound parse contract для completion/diagnostics. `didChange` path уже использует incremental parse с fail-safe full fallback, а syntax diagnostics уже читаются из общего parse state.

Оставшийся пробел не алгоритмический, а наблюдательный: сейчас можно увидеть mode parse snapshot и можно увидеть aggregate latency `syntax_diagnostics_query_ms`, но нельзя формально сравнить syntax-stage latency по parse mode в одном observability contract.

## Goals / Non-Goals
- Goals:
  - Добавить mode-aware observability для syntax diagnostics stage.
  - Сохранить low-cardinality taxonomy и deterministic dual-write semantics.
  - Сделать root-cause сравнение `incremental/reused/full/other` vs syntax latency доступным без новых high-cardinality меток.
- Non-Goals:
  - Менять incremental parse implementation.
  - Менять lifecycle `ParseSnapshot`.
  - Менять user-facing semantics diagnostics.

## Decisions
- Decision 1: Сузить change только до residual observability scope
  - Этот change MUST NOT повторно специфицировать incremental parse, fallback или parse snapshot lifecycle.
  - Единственный scope change: mode-aware измерение latency syntax diagnostics.

  Alternatives considered:
  - Архивировать change без остатка.
    - Отклонено: остаётся практический observability gap для root-cause анализа.
  - Оставить change в исходном виде.
    - Отклонено: это дублирует уже доставленный `ParseSnapshot` contract.

- Decision 2: Использовать ту же mode taxonomy, что и у parse snapshot observability
  - `syntax_diagnostics` MUST публиковать mode-aware latency с теми же значениями: `incremental`, `reused`, `full`, `other`.
  - Источник mode MUST быть тем же parse snapshot / parse-report контуром, который уже используется для diagnostics текущей ревизии.

- Decision 3: Legacy fixed key остаётся aggregate projection
  - `intellisense_v2_syntax_diagnostics_query_ms` MUST сохраниться для backward compatibility.
  - Mode-aware разрез MUST публиковаться через канонический event model и соответствующую deterministic projection, а не через отдельную ad-hoc метрику без taxonomy mapping.

## Risks / Trade-offs
- Риск: mode у syntax-stage будет вычисляться не из того же источника, что и parse snapshot mode.
  - Mitigation: derive mode только из уже выбранного parse snapshot/report для данной ревизии.
- Риск: появится второй observability contract рядом с каноническим event model.
  - Mitigation: расширять только существующий canonical/drilldown contract и legacy projection.
- Риск: рост cardinality.
  - Mitigation: использовать только фиксированный enum `incremental|reused|full|other`.

## Migration / Rollout
1. Добавить mode-aware emission для syntax diagnostics stage в канонический observability pipeline.
2. Сохранить compatibility projection для aggregate fixed-key метрики.
3. Добавить contract/regression tests на mode-aware drilldown и legacy aggregate parity.

## Open Questions
- Нужен ли отдельный fixed-key projection по mode, или для residual scope достаточно canonical/drilldown mode-aware разреза при сохранении aggregate legacy key.
