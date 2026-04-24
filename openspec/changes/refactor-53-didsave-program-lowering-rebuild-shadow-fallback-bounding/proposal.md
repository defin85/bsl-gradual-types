# Change: bound didSave program-lowering rebuild before shadow fallback

## Why

The fresh observability incident bundle captured at `2026-04-24T10:50:21.149Z`
on `0.4.159` / `git 00bcf03f` shows a new residual after
`refactor-52-didsave-parser-base-recovery-detached-ready-bounding`.

`refactor-52` closed the previous started-producer/parser-base contour by adding final
same-family lifecycle evidence and proving cache-disabled/cold-cache runs could stay on
`detached_ready_artifacts`. The new bundle is not that old `parser_base_recovery` failure:
one `didSave` follow-up first proves the healthy path, while the next same-file save times out
inside `parse_exec -> exact_ready_snapshot_assembly -> program_lowering`, performs a full
program-lowering rebuild, falls back to `shadow_state`, and only later records final lifecycle
`detached_diagnostics_ready_published`.

Current evidence:

- completion is not the primary bottleneck: captured completion requests have
  `adapter_to_dispatch_wait_ms=0-1`, `same_file_ingress_token_wait_ms=0`, and
  `response_output_handoff_send_wait_ms=0`;
- `diagnostics-save-trace-1` is the good control: version `11` reaches
  `detached_ready_artifacts`, bounded wait wins in `631ms`, `program_lowering` takes `2ms`, and
  `program_lowering_reuse_outcome=top_level_reuse`;
- `diagnostics-save-trace-2` is the residual: version `15` publishes syntax-only fastlane in
  `50ms`, then bounded wait times out after `3500ms`;
- the timeout is `parse_exec` / `exact_ready_snapshot_assembly` with
  `followup_ready_snapshot_timeout_leaf=program_lowering`;
- `program_lowering` takes `3792ms`, exports `program_lowering_reuse_outcome=full_rebuild`,
  rebuilds `2088` lowering units, reuses `0`, and has both
  `reuse_plan_borrowed_cache_hit=false` and `reuse_plan_take_if_unique_hit=false`;
- terminal diagnostics publication then uses `followup_semantic_path=shadow_state` and spends
  `3679ms` in semantic diagnostics query, while `publish_wait_ms=1`;
- the same trace records final producer lifecycle `detached_diagnostics_ready_published`, so the
  shadow fallback raced a same-family detached-ready producer rather than proving a true non-exact
  terminal outcome.

So the next change should not reopen VS Code UI, transport ingress, waiting-only producer
admission, parser-base recovery, or shadow-state semantic-query optimization. It should bind the
same-version `didSave` producer through the program-lowering rebuild/reuse boundary so a still-current
save family does not fall back through `shadow_state` merely because program-lowering reuse missed and
the detached-ready producer finished after the bounded consumer branch already gave up.

## What Changes

- Add a `bsl-intellisense-v2` requirement that a still-current same-version `didSave` exact
  producer MUST NOT let `parse_exec` / `program_lowering` full rebuild timeout become a normal
  `shadow_state` terminal branch when the same save family later publishes detached diagnostics-ready.
- Tighten representative validation so
  `program_lowering full_rebuild -> bounded wait timeout -> shadow_state -> later detached-ready`
  is a fail gate, separate from the already-closed waiting/parser-base contours.
- Treat bounded-wait expiry and missing program-lowering reuse as insufficient terminal reasons on
  their own when the same save family later proves detached-ready or fully materialized.
- Require the evidence path to preserve `program_lowering` reuse-plan outcome, rebuilt/reused units,
  bounded-wait winner, terminal semantic path, and final same-family lifecycle in the same captured
  cycle.
- Keep detached diagnostics-ready publication as the bounded success endpoint for diagnostics
  follow-up; full live exact install is still not required for this diagnostics-only path.
- Preserve canonical live exact readiness gates for completion, hover, definition, signatureHelp,
  type-at-position, and equivalent interactive exact consumers.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - ready-snapshot producer / parser-edit / program-lowering reuse coordination surfaces
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
  - diagnostics-save timeline and incident-bundle projection if fields are missing from the failing
    branch
- Follow-up relationship:
  - follows `refactor-52-didsave-parser-base-recovery-detached-ready-bounding`;
  - does not reopen `refactor-50` waiting-only fallback or `refactor-52` parser-base recovery;
  - builds on the `refactor-52` cold/cache-disabled lesson that direct same-version `didSave`
    producers must receive enough parser-edit/reuse context to avoid full rebuild;
  - does not reopen completion transport/runtime isolation or VS Code extension pre-send work.

## Non-Goals

- Do not widen the bounded wait or relief-valve budgets as the primary remedy.
- Do not optimize `shadow_state` semantic query first; the steady-state target is to avoid this
  fallback for still-current same-family `didSave` producers.
- Do not make diagnostics-only detached artifacts canonical exact readiness for interactive
  consumers.
- Do not describe this as `refactor-52` unfinished; it is a new rebuild-dominated residual after the
  parser-base contour was closed.
