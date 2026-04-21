# Change: wake same-version `didSave` follow-up on the first matching diagnostics artifact

## Why

The fresh observability incident bundle captured at `2026-04-20T18:37:04.696Z` on
`0.4.158` / `git e7ffc155` confirms that `refactor-44` fixed runtime wiring but did not yet
remove the representative latency coupling.

The new bundle shows:

- the runtime is already able to consume `detached_ready_artifacts` on the live path;
- the successful detached sample still appears only after
  `followup_ready_snapshot_wait_probe=timeout`;
- the same detached sample remains dominated by
  `followup_ready_snapshot_timeout_leaf=ready_install` at `3423ms`;
- a parallel same-file `didSave` sample remains fail-closed on `generic_pipeline` after
  `followup_ready_snapshot_wait_probe=version_mismatch`;
- cumulative metrics now show `followup_semantic_path detached_ready_artifacts=1`,
  `generic_pipeline=1`, `shadow_state=0`.

This changes the diagnosis again.

The remaining gap is not "detached artifacts are missing" and not "interactive exact gates were
weakened". The gap is that the bounded wait still sleeps on canonical ready-snapshot materializa-
tion only, while the detached diagnostics-ready artifact can become available earlier for the same
still-current save target.

So the next change should make the `didSave` heavy follow-up wake on the first matching artifact
that is safe to use:

- canonical `ready_artifacts`, if live exact readiness materializes first;
- detached diagnostics-ready artifacts, if diagnostics-only payload becomes available first;
- otherwise truthful timeout / supersession / cancellation / mismatch behavior.

## What Changes

- Require same-version `didSave` heavy follow-up to treat canonical exact ready artifacts and
  detached diagnostics-ready artifacts as two distinct wake sources for the same still-current
  save target, instead of waiting only on canonical ready materialization until timeout.
- Require the new wait path to stay keyed to
  `(file_id, requested_version, text_hash, save_cycle_sequence)` or a semantically equivalent
  target identity.
- Require canonical live `ready_artifacts` to keep priority whenever it is already materialized or
  wins the bounded race, while detached artifacts remain diagnostics-only and never become proof of
  interactive exact readiness.
- Require observability / incident bundle output to name which wake source won
  (`ready_artifacts`, `detached_ready_artifacts`, or a truthful miss outcome) instead of implying
  that all detached success still came only after a timeout-sized canonical wait.
- Keep completion / transport ingress investigation out of scope for this change; the
  `adapter_to_dispatch_wait_ms=14892` outlier in the same bundle is a separate issue.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/tests/diagnostics_save_timeline/`
  - diagnostics-save observability / incident-bundle projection surfaces
- Follow-up relationship:
  - builds directly on `refactor-44-save-followup-detached-ready-artifacts`
  - does not reopen the `refactor-43-save-critical-parser-base-recovery-bounding` scope
  - remains narrower than `refactor-current-revision-head-detached-snapshot`

## Non-Goals

- Do not widen the bounded wait or relief-valve budgets as the primary remedy.
- Do not weaken canonical live exact-readiness gates for `hover`, `definition`,
  `signatureHelp`, completion exact upgrade, or semantically equivalent interactive consumers.
- Do not replace truthful mismatch / supersession / cancellation outcomes with best-effort detached
  publish.
- Do not solve the separate completion ingress backlog surfaced in the same incident bundle.
