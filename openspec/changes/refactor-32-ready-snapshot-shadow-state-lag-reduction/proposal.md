# Change: reduce ready-snapshot lag so same-file churn stops defaulting to `shadow_state`

## Why

The `2026-04-15T20:23:14Z` incident bundle captured on `0.4.153` / `git 8aa12610` proves that
`refactor-31` fixed the diagnostics-save coherence bug, but it also shows that the remaining
representative `conf_big` bottleneck is no longer about observability correctness.

The new bundle shows:

- the coherence fix is real: both traces now export `program_conversion_ms == program_lowering_ms`
  and the live debug logs stay `incoherent=false`;
- completion transport/UI are still not the primary problem (`0-2ms` empty traces and one
  `166ms` server-side collect path);
- `didSave` still gets a fast first `syntax_only` publish in `43-48ms`;
- the heavy follow-up still publishes through `shadow_state` after `7.0-7.9s`;
- the exact timeout path is still
  `parse_exec -> core_parse_build -> exact_ready_snapshot_assembly -> program_lowering`;
- ranged `didChange` still falls back through `fallback_reason=stale_parser_base` with
  `parser_base_root_cause=ready_snapshot_lags_shadow_state`;
- same-file churn is still wasteful: `ready_snapshot_worker_started did_change=14`,
  `retargeted_during_parse=12`, materialization count only `2`.

That means the next problem is now narrower and different from `refactor-31`:

1. exact same-version work is observable and coherent, but it still lags enough that `didSave`
   follow-up ends in `shadow_state` on the representative mixed profile;
2. the ready head still trails `shadow_state` badly enough that ranged `didChange` keeps treating
   `ready_snapshot_lags_shadow_state` as the steady-state reason for `stale_parser_base`.

## What Changes

- Require same-file `didSave` heavy follow-up to stop treating `shadow_state` as the steady-state
  terminal branch once a still-current exact same-version producer is already inside bounded
  `program_lowering`, without widening the existing wait budgets as the primary fix.
- Require ranged same-file `didChange` churn to keep a parser-base-capable exact head close enough
  to `shadow_state` that `ready_snapshot_lags_shadow_state` stops being the dominant steady-state
  reason for `stale_parser_base` on representative large-module profiles.
- Require targeted regressions and representative `conf_big` live evidence proving that the new
  contract improves exact-head freshness rather than merely re-labeling the same fallback.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`
- `refactor-27-diagnostics-save-exact-parse-exec-bounding`
- `refactor-28-diagnostics-save-exact-core-parse-build-bounding`
- `refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding`
- `refactor-30-diagnostics-save-exact-ready-snapshot-program-conversion-bounding`
- `refactor-31-diagnostics-save-exact-program-lowering-bounding`

`refactor-31` truthfully localized the remaining exact residual to bounded `program_lowering` and
fixed the broken conversion attribution. The current bundle shows the next real problem: exact-head
freshness under same-file churn is still not strong enough to return the representative mixed path
to `ready_artifacts`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `analysis-v2` / current ready-head publication and reuse boundaries
  - representative perf/live evidence for `examples/conf_big`

## Non-Goals

- Do not reopen the already fixed diagnostics-save coherence bug.
- Do not widen `didSave` bounded wait or relief-valve budgets as the primary remediation.
- Do not shift the investigation back to VS Code UI or completion transport for this incident
  class.
- Do not relax latest-wins semantics or publish stale diagnostics just to avoid `shadow_state`.
