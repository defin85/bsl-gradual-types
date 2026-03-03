# Change: Full rewrite observability/perf pipeline для IntelliSense v2

## Why
Текущий observability/perf слой содержит рабочие механизмы, но архитектурно остаётся фрагментированным:
- instrumentation частично разнесён по adapter/runtime слоям;
- projection/normalization эволюционировали инкрементально и усложняют долгосрочную эволюцию taxonomy;
- perf evidence provenance и acceptance-gate логика остаются чувствительными к drift между runtime/tests/scripts.

Для production-эксплуатации с долгим lifecycle нужен отдельный change на полный rewrite pipeline с единой архитектурной границей и явной migration стратегией.

## What Changes
- **ADDED**: архитектурный контракт `observability/perf pipeline v3`:
  - единый canonical event ingestion pipeline;
  - registry-compiled materialization (canonical -> legacy/drilldown);
  - centralized instrumentation API для LSP/web/MCP/runtime;
  - fail-closed perf evidence provenance envelope и validator boundary.
- **ADDED**: rollout/migration требования для dual-write, canary, rollback и controlled legacy deprecation.
- **ADDED**: delivery ownership/gates требования, исключающие adapter-local bypass и частичную семантику.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/application/intellisense_v2/*`
  - `backend/src/bin/lsp_server/server/*`
  - `backend/src/perf_gate_evaluator.rs`
  - `backend/src/bin/intellisense_perf.rs`
  - `contracts/observability-completion-v2/*`
  - `contracts/intellisense-perf-gate/*`
  - `scripts/check-versioned-contracts.py`

## Relation To Existing Changes
- `refactor-v2-contract-first-hardening` остаётся отдельным и явно зафиксированным recommended hardening path.
- Этот change описывает отдельный full rewrite трек с иным масштабом и рисками, и не отменяет hardening change автоматически.
