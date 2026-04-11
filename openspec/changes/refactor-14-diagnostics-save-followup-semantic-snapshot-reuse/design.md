## Context

После `refactor-08/09/10` didSave pipeline уже имеет:

- bounded `save_fastlane` first publish;
- same-version syntax reuse для `idle_heavy`;
- dedicated `did_save_followup` lane и truthful request-centric save timeline.

Но bundle `2026-04-11T16-14-14Z` показывает, что remaining tail всё ещё material:

- `save_fastlane` публикуется за `43-53ms`;
- `idle_heavy` follow-up публикуется за `3629ms` и `9293ms`;
- `semantic_diagnostics_query_ms` в follow-up = `3011-3301ms`;
- внутри semantic diagnostics `parse_result_ms p95=3011ms`, `ir_ms p95=469ms`,
  `collect_ms p95=162ms`.

Read-only code analysis указывает на следующий architectural gap:

1. `AnalysisV2::parse_result()` уже умеет брать version-bound `parse_snapshot`.
2. `AnalysisV2::ir_profiled()` уже умеет reuse exact cache и parse-snapshot-backed IR build.
3. Но `semantic_diagnostics_profiled()` вызывает direct salsa `parse_result(...)` и `ir(...)`
   вместо snapshot-aware wrapper accessors.
4. `didSave` follow-up сперва идёт через `shadow_state` semantic path, а `ready_artifacts`
   path вызывается только после него.

Итог: even when same-version ready parse snapshot already exists, semantic follow-up still
behaves too much like a cold parse/IR path.

## Goals

- Remove redundant same-version semantic parse/IR recompute from didSave heavy follow-up when
  same-version ready parse snapshot is already available.
- Prefer the already materialized ready-artifacts branch over shadow-state semantic work when that
  decision can be made immediately.
- Keep the optimization fail-closed and same-version-correct.
- Preserve operator-facing observability so the optimization can be proven from request-centric
  traces instead of inferred from aggregate latency alone.

## Non-Goals

- Do not widen the fix into a general-purpose salsa memoization redesign.
- Do not change `save_fastlane`.
- Do not remove the dedicated `did_save_followup` lane or alter its quota semantics.
- Do not depend on a long wait budget for ready artifacts before fallback.

## Decisions

### 1. Semantic diagnostics profile must use snapshot-aware analysis accessors

The primary fix belongs in `analysis-v2`, not only in diagnostics routing.

`semantic_diagnostics_profiled()` and `semantic_diagnostics_flow_sensitive_profiled()` should stop
calling direct salsa `parse_result(...)` / `ir(...)` as their profiled primitives. Instead, they
should use snapshot-aware `AnalysisV2` accessors:

- parse via `AnalysisV2::parse_result(...)` or semantically equivalent snapshot-aware wrapper;
- IR via `AnalysisV2::ir_profiled(...)` or semantically equivalent snapshot/cache-aware wrapper.

This keeps the existing source of truth inside `AnalysisV2` and avoids introducing ad-hoc external
memo injection.

Rejected alternative: external `salsa::specify`-style injection of parse artifacts into tracked
queries. This would complicate correctness, ownership, and revision semantics far more than needed
for this scoped fix.

### 2. didSave follow-up should prefer ready artifacts immediately when already ready

`didSave + idle_heavy` should not always enter the `shadow_state` semantic branch first.

Preferred branch order:

1. if same-version ready parse snapshot already exists at follow-up start, use `ready_artifacts`;
2. otherwise use `shadow_state`;
3. if neither is provably fresh, fall back to the existing generic pipeline.

This is intentionally an immediate preference, not a new long wait. The system should not insert a
fresh seconds-scale or multi-hundred-millisecond budget before fallback just to wait for ready
artifacts.

### 3. Observability must expose semantic path and artifact source

Latency improvement alone is not enough; the request-centric trace must show whether the optimized
path actually fired.

The bounded taxonomy for follow-up semantic attribution should be:

- `followup_semantic_path = ready_artifacts | shadow_state | generic_pipeline`
- `followup_semantic_parse_source = snapshot | salsa`
- `followup_semantic_ir_source = exact_cache | snapshot_build | salsa`

Equivalent names are acceptable only if the taxonomy remains bounded and operator-meaningful.

### 4. Fallback stays fail-closed

The optimization is allowed only when same-version freshness is provable.

If ready parse snapshot is absent, stale, or mismatched:

- do not publish stale diagnostics;
- fall back to current truthful shadow/generic path;
- keep supersession/latest-wins semantics intact.

### 5. Diagnostics save timeline contract bump stays explicit

`bsl.getDiagnosticsSaveTimeline` is already a versioned authoritative payload consumed by the
VS Code extension and incident bundle export, so semantic path/source attribution cannot be treated
as an invisible internal field addition.

This change should therefore ship as additive diagnostics-save-timeline contract bump `v7 -> v8`.

`v8` MUST include bounded semantic follow-up attribution sufficient for operator workflows:

- `followup_semantic_path = ready_artifacts | shadow_state | generic_pipeline`
- `followup_semantic_parse_source = snapshot | salsa`
- `followup_semantic_ir_source = exact_cache | snapshot_build | salsa`

Older consumers remain supported only via explicit degradation:

- if `response.version < 8`, extension and incident-bundle surfaces MUST say semantic
  path/source attribution is unavailable by design;
- they MUST NOT infer missing semantic reuse facts from absence of the new fields or from aggregate
  latency alone;
- raw `v7` payload remains valid but does not satisfy this change's stronger operator-facing
  observability contract.

## Validation Strategy

1. Regression: snapshot-backed semantic diagnostics profile no longer forces direct full
   `parse_result` recompute when same-version `SetFileWithSnapshot` state is available.
2. Regression: didSave follow-up prefers immediate `ready_artifacts` branch over `shadow_state`
   when same-version ready parse snapshot already exists.
3. Regression: missing or stale snapshot still falls back truthfully and preserves correctness.
4. Regression: diagnostics save timeline `v8` and extension/bundle consumers expose bounded
   semantic path/source attribution, while `v7` degrades explicitly as unavailable-by-design.
5. Live evidence: representative `conf_big` save flow shows reduced
   `semantic_diagnostics_query_parse_result_ms` and explicit semantic source attribution.
