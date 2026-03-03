## Context
`refactor-v2-contract-first-hardening` уменьшает drift-риск в существующей архитектуре, но не устраняет полностью историческую фрагментацию instrumentation/projection/perf-gate границ.

Для долгосрочной production-эксплуатации требуется отдельный rewrite-трек, который:
- задаёт единую архитектурную границу observability/perf pipeline;
- исключает adapter-local semantic divergence by design;
- приводит perf evidence к воспроизводимому fail-closed provenance контракту.

## Goals / Non-Goals
- Goals:
  - Ввести единый pipeline `ingest -> validate -> project -> export`.
  - Сделать projection deterministic и registry-compiled.
  - Централизовать instrumentation API для всех adapter/runtime путей.
  - Сделать perf evidence acceptance fail-closed на provenance mismatch/missing.
  - Обеспечить контролируемую миграцию через dual-write/canary/rollback.
- Non-Goals:
  - Изменение user-facing LSP protocol/wire behavior.
  - Редизайн type inference, ranking или semantic алгоритмов.
  - Миграция несвязанных подсистем вне observability/perf boundary.

## Target Architecture
### 1) Pipeline Core Boundary
Единый runtime pipeline принимает canonical events и выполняет:
1. schema validation (allowed dimensions/families/value kinds);
2. registry lookup;
3. deterministic materialization;
4. export routing.

Прямой bypass в export слой считается архитектурным нарушением.

### 2) Registry-Compiled Projection
Taxonomy (`operation/stage/reason/outcome`) задаётся typed registry.
Из registry строятся:
- canonical normalization;
- legacy/drilldown projection targets;
- contract completeness checks.

Добавление taxonomy значений без полного mapping MUST блокироваться в CI.

### 3) Centralized Instrumentation API
Adapters (LSP/web/MCP/runtime) используют единый instrumentation API и не формируют metric keys вручную.
Все user-facing interactive операции (`completion/hover/signatureHelp/definition`) проходят через общий emission path.

### 4) Perf Evidence Provenance Boundary
Perf artifacts включают обязательный provenance envelope:
- `change_id`;
- `generated_at`;
- `profile`;
- `schema_version`;
- `contract_version`.

Validator работает fail-closed: mismatch/missing обязательных provenance полей => invalid evidence.

### 5) Migration Strategy
Rollout по фазам:
1. dual-write (legacy + v3 pipeline);
2. canary parity checks;
3. cutover на v3 primary;
4. controlled deprecation legacy paths.

Rollback сохраняется до завершения фазы cutover.

## Alternatives Considered
- Оставаться только на contract-first hardening.
  - Rejected for this change: снижает риск, но не решает архитектурную фрагментацию полностью.
- Big-bang switch без dual-write.
  - Rejected: неприемлемый operational risk для production rollout.

## Risks / Trade-offs
- Риск: высокий объем изменений и рост initial complexity.
  - Mitigation: phased rollout + strict ownership boundaries + incremental migration gates.
- Риск: временный рост cost/overhead в dual-write фазе.
  - Mitigation: ограниченная длительность dual-write и регулярный parity audit.
- Риск: contract version drift между runtime/scripts/CI.
  - Mitigation: versioned contracts + single validator boundary + fail-closed checks.

## Migration Plan
1. Зафиксировать spec delta и архитектурные контракты.
2. Реализовать pipeline core и registry-compiled materialization.
3. Перевести adapters на centralized instrumentation API.
4. Включить provenance envelope и fail-closed validator.
5. Пройти dual-write/canary, затем cutover и controlled legacy deprecation.

## Open Questions
- Нужен ли отдельный major surface (`observability-completion-v3`) или достаточно major bump существующего surface при строгой migration note.
- Какой target window dual-write допустим по production cost (SLO/cost budget).
