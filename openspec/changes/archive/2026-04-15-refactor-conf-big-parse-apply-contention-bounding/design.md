## Context
`conf_big` incident bundle от `2026-04-09` показал две независимые, но усиливающие друг друга
проблемы:

1. same-file waiters на current-revision visibility (`wait_for_file_version`, `apply_lag`) получают
   multi-second tail даже когда truthful transport seams остаются healthy;
2. same-version parse consumers на большом тексте периодически запускают cold/full parse path и
   делают это в общем bounded background CPU domain с общей parser serialization.

Completion traces в bundle сами по себе не указывают на UI/transport bottleneck. Основной material
latency сидит в backend runtime, parse snapshot build и didSave follow-up.

## Goals
- Защитить current-revision applied-state visibility от same-file auxiliary parse churn.
- Сделать same-version parse truth reuse/coalescing обязательным для large-module auxiliary paths.
- Убрать repeated independent cold/full parse как default behavior для идентичного same-version text.
- Сделать representative acceptance способной различать parse-cold-start regression и writer/apply
  backlog regression.

## Non-Goals
- Не менять user-facing semantic contract `bsl.getCurrentContext`, `documentSymbol`, completion или
  diagnostics beyond latency/readiness behavior.
- Не делать detached immutable head snapshot в рамках этого change.
- Не переписывать весь observability/perf pipeline.
- Не полагаться на UI-side heuristics вместо server/runtime remediation.

## Decisions

### 1. Current-revision `SetFile` visibility становится same-file protected progression point
После того как same-file handoff для revision `V` уже зарегистрирован через `didOpen`, `didChange`
или `didSave`, наблюдаемое продвижение `applied_version >= V` должно рассматриваться как
same-file critical progression point.

Auxiliary same-file work может продолжаться в фоне, но не должна по умолчанию оставлять newest
same-file waiters в состоянии "handoff был, а applied visibility все еще не наблюдаема" только
потому, что впереди стоят parse snapshot, current-context parse, type-index precompute или другой
same-file auxiliary churn.

### 2. Same-version parse truth должна быть shareable/coalesced boundary
Для large modules один и тот же `(file_id, file_version, text_hash)` не должен порождать repeated
independent cold/full parse by default.

Implementation может использовать singleflight, parse snapshot reuse, cache promotion или другой
эквивалентный механизм, но contract один:
- один cold/full parse допускается, если нет предыдущего дерева или incremental basis;
- последующие same-version consumers reuse/coalesce existing truth, а не заново платят ту же цену.

Это особенно важно для:
- `build_parse_snapshot_v2`;
- save-triggered same-version refresh;
- `bsl.getCurrentContext` parse/context derivation.

### 3. Один parser mutex не должен оставаться latency amplifier без bounded mitigation
Текущий глобальный parser serialization path допустим только если поверх него есть bounded reuse,
singleflight или другой механизм, который не превращает один slow full parse в серию независимых
same-version tails.

Change не навязывает конкретный implementation shape вроде "несколько parser instances", но
требует убрать нынешний эффект, когда один cold parse автоматически раздувает latency нескольких
same-file auxiliary consumers.

### 4. Acceptance должна разделять parse-cold-start и writer/apply backlog
Representative `conf_big` validation должна иметь как минимум один same-file mixed-load profile,
который одновременно упражняет:
- `didChange`;
- `didSave`;
- auxiliary parse-only load (`bsl.getCurrentContext` или эквивалент);
- waiter на current-revision visibility (`completion` и/или didSave heavy follow-up).

Gate должен отдельно видеть:
- repeated `mode_full` / fallback regression;
- `apply_changes` / `wait_for_file_version` / `apply_lag` regression.

## Alternatives Considered

### Добавить только observability
Отклонено. Bundle уже дал достаточную root-cause direction; проблема не в нехватке общих метрик,
а в runtime/parse behavior.

### Сразу делать detached immutable current-revision snapshot
Отклонено как более широкий архитектурный трек. Этот change про ближайшее remediation current
runtime/parse contention, а не про полную read-model эволюцию.

### Просто увеличить background capacity
Отклонено. Это маскирует contention, но не фиксирует repeated identical full parse и не делает
current-revision visibility first-class protected progression point.

## Risks / Trade-offs
- Риск: same-version reuse/coalescing усложнит invalidation semantics.
  - Mitigation: ключевать reuse по `(file_id, file_version, text_hash)` или семантически
    эквивалентному identity.
- Риск: bounded mitigation уменьшит parallel throughput для unrelated background work.
  - Mitigation: change целится именно в same-file same-version duplication и apply visibility, а не
    в global starvation of all background work.
- Риск: acceptance overfit-ится на `conf_big`.
  - Mitigation: использовать `conf_big` как representative large-module fixture, но формулировать
    spec через class of behavior, а не через fixture-specific hack.

## Open Questions
- Нужна ли explicit diff derivation для full-text `didChange`, или same-version coalescing/reuse
  уже достаточно для первых remediation шагов?
