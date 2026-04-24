# Change: bound didSave exact materialization latency after detached-ready recovery

## Why

The fresh observability incident bundle captured at `2026-04-24T14:22:42.992Z`
on `0.4.160` / `git 00bcf03f` shows a new residual after
`refactor-53-didsave-program-lowering-rebuild-shadow-fallback-bounding`.

`refactor-53` targeted the correctness failure where a still-current same-version `didSave`
follow-up timed out inside `program_lowering`, published through `shadow_state`, and only later
proved that the same save family had reached detached diagnostics-ready. The new bundle does not
show that terminal `shadow_state` failure. Both captured save follow-ups publish through
`detached_ready_artifacts`. The remaining incident is latency inside the exact materialization
producer and first-publish syntax path:

- completion is not the primary bottleneck: captured completion requests have
  `client_before_transport_write_wait_ms=1-2`, `scheduler_poll_ready_wait_ms=0`,
  `admission_queue_wait_ms=0`, `same_file_ingress_token_wait_ms=0`, and
  `response_output_handoff_send_wait_ms=0-1`; the largest completion is `190ms`, dominated by
  `collect`;
- `diagnostics-save-trace-1` publishes the heavy follow-up through `detached_ready_artifacts` in
  `577ms`, but its first `save_fastlane` syntax-only publish takes `3397ms` and is dominated by
  `syntax_diagnostics_query_ms=3397` while the same-family exact producer spends `3926ms` in
  `parser_base_recovery`;
- `diagnostics-save-trace-2` publishes `save_fastlane` quickly (`55ms`) and later publishes heavy
  follow-up through `detached_ready_artifacts`, but only after `4884ms`;
- that second trace times out bounded wait after `3502ms`, times out relief valve after `501ms`,
  and records `parse_exec -> exact_ready_snapshot_assembly -> program_lowering` as the dominant
  blocker with `program_lowering=4230ms`;
- the same second trace exports `program_lowering_reuse_outcome=full_rebuild`, `2088` rebuilt
  lowering units, `0` reused units, and no borrowed-cache or take-if-unique reuse hit;
- both traces ultimately report final lifecycle `detached_diagnostics_ready_published`.

So the next change should not reopen VS Code UI dispatch, transport ingress/egress, waiting-only
producer admission, or `shadow_state` fallback correctness. It should treat exact materialization
latency itself as the residual: first-publish syntax recomputation can still take seconds, and a
heavy follow-up can still miss the bounded window because the still-current exact producer performs
a full `program_lowering` rebuild before detached-ready publication.

## What Changes

- Add `bsl-intellisense-v2` requirements that detached diagnostics-ready terminal publication is
  necessary but not sufficient: a same-version `didSave` follow-up that publishes through
  `detached_ready_artifacts` after bounded wait and relief-valve timeouts is still a latency
  failure unless the runtime exports a truthful supersession, cancellation, failure, or continuity
  loss reason.
- Add a first-publish requirement so `save_fastlane` syntax-only refresh remains bounded when the
  same-family exact producer is still in parser-base recovery, or exports a truthful first-publish
  blocker instead of being hidden by a later successful heavy follow-up.
- Add representative validation for the new non-shadow residual:
  `detached_ready_artifacts` terminal path plus `program_lowering full_rebuild`, `0` reused units,
  bounded-wait timeout, relief-valve timeout, and multi-second full follow-up elapsed time.
- Preserve the `refactor-53` correctness gate: terminal `shadow_state` fallback for the same family
  remains invalid, but avoiding `shadow_state` alone does not close this latency class.
- Preserve low-cardinality evidence for save-cycle identity, first-publish syntax timing, exact
  phase attribution, `program_lowering` reuse/rebuild counts, bounded wait and relief-valve
  outcomes, terminal semantic path, and final same-family lifecycle.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - ready-snapshot producer / parser-base recovery / program-lowering reuse coordination surfaces
  - diagnostics-save timeline export and incident-bundle projection
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
  - targeted diagnostics-save timeline regressions
- Follow-up relationship:
  - follows `refactor-53-didsave-program-lowering-rebuild-shadow-fallback-bounding`;
  - does not describe `refactor-53` as unfinished, because the fresh bundle does not show terminal
    `shadow_state` fallback;
  - does not reopen `refactor-50`, `refactor-51`, or `refactor-52` waiting/parser-base terminal
    fallback contours;
  - keeps completion transport/runtime isolation and VS Code extension pre-send work out of scope
    unless a newer bundle contradicts the current timing evidence.

## Non-Goals

- Do not widen bounded wait or relief-valve budgets as the primary remedy.
- Do not optimize `shadow_state` semantic query first; the fresh bundle's heavy follow-up terminal
  path is already `detached_ready_artifacts`.
- Do not weaken canonical live exact readiness gates for completion, hover, definition,
  signatureHelp, type-at-position, or semantically equivalent interactive exact consumers.
- Do not call current local refactor-53 work accepted from this bundle alone; the bundle was
  captured from an installed `git 00bcf03f` binary, not necessarily from the current dirty worktree.
