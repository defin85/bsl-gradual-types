## Context
Текущий v2 pipeline имеет признаки heavy parse cost under churn на больших модулях:
- в `large/warm` доминируют `syntax_diagnostics_query` и `ir_query_completion`;
- queue wait latency существенно ниже query latency.

Это указывает на алгоритмический bottleneck parse/query-path, а не на saturation только runtime очередей.

## Goals / Non-Goals
- Goals:
  - Снизить стоимость parse path при частых `didChange` на больших документах.
  - Обеспечить детерминированный version-bound parse state для completion/diagnostics.
  - Сохранить корректность и strict latest-version контракты.
- Non-Goals:
  - Менять пользовательскую семантику BSL diagnostics.
  - Полностью переписывать AST/IR доменную модель.

## Decisions
- Decision 1: Ввести `ParseSnapshot` как явный runtime контракт
  - Snapshot содержит tree-sitter tree, parse_result, line_index, changed_ranges, produced_version.
  - Snapshot всегда привязан к конкретной версии файла и не может быть использован cross-version без явной stale policy.

  Alternatives considered:
  - Оставить parse как чисто lazy query без stateful snapshot.
    - Отклонено: плохо контролируется стоимость under churn и нет явного reuse контракта.

- Decision 2: Обновлять snapshot инкрементально по edit цепочке
  - На `didChange` использовать `old_tree + InputEdit + parse(new, Some(old_tree))`.
  - При mismatch или failure — fail-safe full parse с фиксированной observability причиной.

  Alternatives considered:
  - Делать incremental только для diagnostics, а completion оставить на full parse.
    - Отклонено: не решает latency bottleneck completion path.

- Decision 3: changed-ranges aware invalidation
  - Downstream стадии получают range-дельту и могут ограничивать пересчет.
  - При сомнении корректности — деградация в полный пересчет.

## Risks / Trade-offs
- Риск: некорректное UTF-16/byte mapping в edit-конвертации.
  - Mitigation: property-based и regression тесты для edit sequences.
- Риск: рост памяти из-за хранения tree/snapshot для больших файлов.
  - Mitigation: bounded cache + eviction policy + observability по memory footprint.
- Риск: сложность интеграции с существующим salsa query graph.
  - Mitigation: поэтапный rollout через shadow/compare режим.

## Migration Plan
1. Включить snapshot слой в report-only режиме (метрики + parity checks).
2. Включить incremental path для completion/diagnostics на ограниченном canary.
3. Переключить на default после прохождения churn perf gate.

## Open Questions
- Нужен ли отдельный range-threshold, после которого выгоднее сразу full parse.
- Достаточно ли file-level snapshot, или требуется statement-level shard для IR invalidation.
