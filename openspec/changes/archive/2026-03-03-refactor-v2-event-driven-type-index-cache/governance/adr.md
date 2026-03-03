# ADR: Event-driven precompute `type_index` + serve-only interactive path

## Status
accepted

## Change ID and Criticality
- change_id: `refactor-v2-event-driven-type-index-cache`
- change_criticality: `perf_critical`

## Context
Текущий интерактивный path периодически выполняет on-demand parse/type-index compute под churn-нагрузкой,
что приводит к секундным p99 tail latency на больших модулях.

Цель change: вынести тяжелый compute в ingest-time precompute (`didOpen/didChange`) и оставить request-time
path только на serve-only cache с bounded degraded outcomes.

## Options Considered
1. Incremental hotfix в request path (`compute on miss`) без архитектурного разделения.
2. Event-driven precompute + serve-only cache path с latest-wins и bounded fallback.

## Decision
Принята option 2:
- `type_index` вычисляется только на `didOpen/didChange` precompute jobs;
- interactive path (`completion/hover/signatureHelp`) читает только precomputed artifacts;
- sync parse/index compute в interactive path запрещен.

## Budgets
- Latency: отсутствие seconds-scale хвостов в interactive path under churn.
- Resource: bounded artifact retention (`N=2` per-file window, global guard `MAX_ARTIFACTS=10_000`).
- Contention: latest-wins supersede/cancel под burst `didChange`.

## Validation Plan
- parity tests legacy vs serve-only на одинаковых `(version,deps,settings)`;
- supersede/cancel tests under burst `didChange`;
- perf checks `small|large|churn` с акцентом на tail latency;
- observability proof, что interactive path не исполняет on-demand parse/index.

## Rollback / Supersede
- rollout по mode flags: `shadow -> canary -> on`;
- rollback: возврат к legacy mode без API-изменений wire-контракта.
- supersede policy: новый ADR + update contract/spec deltas.

## Owners and Approvers
- ADR Owner: backend/runtime architecture.
- Perf Budget Owner: intellisense-v2 owners.
- LSP Owner: lsp_server owners.
