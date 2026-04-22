# Change: reduce exact `program_lowering` cost via changed-range-aware lowering reuse

## Why

The incident bundle captured at `2026-04-15T22:59:27Z` on `0.4.153` / `git c172fe76` shows that
`refactor-32` fixed the previous same-file freshness problem for the representative `conf_big`
profile:

- ranged `didChange` no longer exports `stale_parser_base`;
- `didSave` heavy follow-up no longer defaults to `shadow_state`;
- completion transport / UI pre-send are still effectively clean (`client_before_transport_write_wait_ms=1-2`,
  `transport_to_method_wait_ms<=1`, `response_output_handoff_send_wait_ms<=1`).

What remains is narrower and now looks like raw backend work rather than another orchestration bug:

- both representative `didSave` follow-ups still spend about `2569-2573ms` inside
  `parse_exec -> core_parse_build -> exact_ready_snapshot_assembly -> program_lowering`;
- the dominant checkpoint is still `program_lowering`;
- the exact path already publishes through `ready_artifacts`, so another fallback-routing refactor
  would mostly relabel a real CPU hotspot rather than remove it.

The current implementation already supports cooperative checkpoints and a narrow append-style
`reused_prefix` fast path, but the new bundle strongly suggests that local same-file edits still
pay too much monolithic lowering work inside large callable bodies.

## What Changes

- Require the exact ready-snapshot path to reuse unchanged lowering units conservatively for local
  same-file edits, so `program_lowering` cost drops because less work is performed, not because the
  wait policy changes again.
- Require reuse planning to stay fail-closed: when invalidation boundaries are ambiguous, the
  runtime MUST rebuild the affected lowering region rather than publish stale exact artifacts.
- Require observability that distinguishes reused versus rebuilt lowering work, so representative
  `conf_big` bundles can prove that the residual moved from "full lowering of almost everything" to
  "bounded rebuild of the changed region".
- Require targeted regressions and refreshed representative live evidence against the current
  `c172fe76` baseline.

## Sequence

This change intentionally follows:

- `refactor-31-diagnostics-save-exact-program-lowering-bounding`
- `refactor-32-ready-snapshot-shadow-state-lag-reduction`

`refactor-31` made `program_lowering` bounded and attribution-coherent.
`refactor-32` made same-file exact-head freshness good enough that representative follow-ups now
return through `ready_artifacts`.
The next step is therefore not another orchestration fix. It is to reduce the actual lowering work
performed on the exact path for local same-file churn.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `syntax/src/tree_sitter_adapter/mod.rs`
  - `syntax/src/tree_sitter_adapter/statement_converter/mod.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - exact-path observability / representative perf evidence

## Non-Goals

- Do not widen bounded wait or relief-valve budgets as the primary fix.
- Do not re-open `shadow_state` routing or `stale_parser_base` recovery as the primary target.
- Do not re-open VS Code UI or transport investigation without contradictory fresh evidence.
- Do not relax exact same-version guarantees, publish stale diagnostics, or silently downgrade the
  semantic contract.
- Do not treat the one slow first `save_fastlane` syntax publish in the new bundle as part of this
  change unless new evidence proves it shares the same root cause.
