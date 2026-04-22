# Change: bound same-version didSave ready-snapshot rebuild before shadow-state fallback

## Why

The fresh observability incident bundle captured at `2026-04-22T10:25:01.243Z` on
`0.4.159` / `git b050f812` changes the diagnosis again.

The old `refactor-48` residual is not what this bundle shows:

- same-file completion no longer burns time in `same_file_ingress_token_wait_ms`;
- `completion_barrier_active_at_dequeue=false` on the representative non-empty completion traces;
- `adapter_to_dispatch_wait_ms=0` and `service_future_to_first_poll_wait_ms=0` on the same path;
- the only visible completion outlier is `completion-trace-5`, dominated by handler-local
  `collect=254ms`, not by ingress or transport backlog.

The remaining incident now sits in `didSave` heavy follow-up.

The same bundle shows:

- `diagnostics-save-trace-1` still ends through `followup_semantic_path=shadow_state` after
  `followup_publish.elapsed_ms=18085ms`, with `runtime_queue_wait_ms=9521` and
  `semantic_diagnostics_query_ms=8449`;
- `diagnostics-save-trace-2` also ends through `followup_semantic_path=shadow_state` after
  `followup_publish.elapsed_ms=12663ms`, with `followup_apply_lag_ms=5351`,
  `followup_ready_snapshot_task_state=in_flight_same_version`,
  `followup_ready_snapshot_wait_probe=timeout`,
  `followup_ready_snapshot_timeout_phase=parse_exec`, and
  `followup_ready_snapshot_timeout_leaf=program_lowering`;
- the same trace spends `followup_ready_snapshot_parse_exec_ms=4083ms`, of which
  `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=4018ms`;
- cumulative metrics show `followup_semantic_path shadow_state=2` and
  `ready_snapshot_materialization source=did_save p50/p95=21168ms`.

So the next change should not reopen the didChange handoff work and should not blame VS Code UI.
It should narrow the scope to the still-current same-version `didSave` ready-snapshot rebuild
path itself: the saved revision is already known, but exact ready-snapshot rebuild can still spend
the whole bounded window inside `parse_exec -> exact_ready_snapshot_assembly -> program_lowering`
and force `shadow_state`.

## What Changes

- Add a `bsl-intellisense-v2` requirement that same-version `didSave` heavy follow-up MUST NOT
  default to seconds-scale cold exact ready-snapshot rebuild when the same saved revision is still
  current and the server already has matching current-revision state that can safely seed a faster
  exact rebuild path.
- Require a reuse-aware or semantically equivalent same-version rebuild fast path for the exact
  ready-snapshot producer, keyed to the exact save target identity, while preserving canonical
  exact truth and latest-wins semantics.
- Require representative `examples/conf_big` validation to fail if heavy follow-up still lands on
  `shadow_state` only because same-version rebuild stayed dominated by
  `parse_exec/exact_ready_snapshot_assembly/program_lowering` rather than by newer revision
  supersession or a separately attributed queue/apply blocker.
- Keep request-centric diagnostics-save evidence truthful about whether the remaining blocker, if
  any, is rebuild-stage latency versus queue/apply lag.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `backend/src/bin/lsp_server/server/core/tests/diagnostics_save_timeline/`
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
- Follow-up relationship:
  - builds on `refactor-44-save-followup-detached-ready-artifacts`
  - builds on `refactor-46-save-followup-dual-artifact-wait`
  - explicitly does not reopen `refactor-48-didchange-current-revision-handoff-fast-lane`
  - remains narrower than `refactor-current-revision-head-detached-snapshot`

## Non-Goals

- Do not widen bounded wait or relief-valve budgets as the primary remedy.
- Do not weaken canonical interactive exact-readiness gates for completion, hover, definition,
  signatureHelp, or semantically equivalent exact consumers.
- Do not treat `shadow_state` as canonical exact truth for the saved revision.
- Do not mix this change with generic transport/UI investigation or the older didChange handoff
  residual.
