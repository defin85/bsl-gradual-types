# Change: preserve didSave lowering reuse continuity

## Why

The fresh observability bundle
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T11-07-23Z`
was captured from runtime git `5691e618`, after
`refactor-59-didsave-program-lowering-tail-bounding` and the current-context
same-text ready-snapshot follow-up fix were installed. It is therefore a valid
post-refactor-59 observation, not an installed-runtime mismatch.

The bundle proves the previous change helped and that the remaining problem is
narrower:

- completion is healthy: six traces, `completion_duration_ms p95=203ms`,
  `client_before_transport_write_wait_ms=2-5ms`, output handoff/write waits
  `<=1ms`, and completion fallback/stale counters are `0`;
- observability integrity is clean:
  `intellisense_v2_observability_contract_violation_total=0` and invalid
  saturation metrics absent;
- one same-file `didSave` follow-up proves the desired reuse shape:
  requested version `11`, first publish `65ms`, full follow-up `2258ms`,
  `program_lowering=1ms`, `reuse_outcome=top_level_reuse`,
  `reuse_plan_build_source=borrowed`, `2088` lowering units reused and `0`
  rebuilt;
- a later same-file `didSave` follow-up still falls off the reuse path:
  requested version `15`, first publish `62ms`, full follow-up `4649ms`,
  blocker `program_lowering_tail`, timeout leaf `program_lowering`,
  `program_lowering=4125ms`, `reuse_outcome=full_rebuild`, `0` units reused and
  `2088` rebuilt.

The v15 failure is not that the runtime cannot classify `program_lowering_tail`
anymore. It is that the exact producer reaches program lowering with no reuse
plan source: `reuse_plan_build_source=null`, `take_if_unique_hit=false`, and
`borrowed_cache_hit=false`. In local code, exact lowering reuse currently starts
from the parser coordinator AST cache for the previous source text. That makes
reuse continuity opportunistic under concurrent same-file work: if the previous
parse result is not present under the expected key, exact assembly silently
degrades to full rebuild.

The next change should make the save-family lowering reuse seed explicit,
durable for the active save contour, and fail-visible when it cannot be used.

## What Changes

- Add a `bsl-intellisense-v2` requirement that same-file `didSave` exact
  ready-snapshot assembly MUST preserve lowering reuse continuity across the
  save family when a prior ready snapshot, didChange parse snapshot, or
  equivalent same-text/same-family source is available.
- Require full rebuild to be a truthful outcome, not an implicit fallback:
  `reuse_plan_build_source=null` with `full_rebuild` MUST carry a
  low-cardinality reason such as missing seed, text-hash mismatch, missing or
  unsafe changed ranges, syntax-tree incompatibility, supersession,
  cancellation, failure, or continuity loss.
- Introduce an architecture boundary between an opportunistic parser AST cache
  and a save-family lowering reuse seed. The save-critical path may use the
  cache, but acceptance cannot depend solely on cache residency when a safer
  request-local or file-local seed exists.
- Add representative validation driven by the `2026-04-27T11-07-23Z` bundle:
  the successful v11 reuse case must remain fast, while the v15 unproved
  `0/2088` full rebuild and seconds-scale program-lowering tail must be fixed
  or classified with a required-full-rebuild reason.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - ready-snapshot exact assembly / lowering reuse seed selection
  - didChange/didSave parse-snapshot and save-family lifecycle wiring
  - `backend/src/bin/lsp_server/server/core.rs`
  - diagnostics-save timeline reason fields and incident-bundle projection
  - targeted parser-coordinator and diagnostics-save timeline tests
  - representative `conf_big` didSave follow-up validation reports

## Non-Goals

- Do not reopen VS Code UI/pre-send, completion ingress, output handoff, or
  runtime saturation for this bundle.
- Do not solve the residual by widening bounded wait or relief-valve budgets.
- Do not make detached diagnostics-ready artifacts canonical exact readiness for
  interactive consumers.
- Do not require global, unbounded AST retention. The seed must be scoped,
  bounded, and tied to active file/save-family identities.
- Do not hide required full rebuilds. If a full rebuild is genuinely required,
  the exported evidence must prove why.
