# Change: fast-lane same-file didChange current-revision handoff before full handler work

## Why
The incident bundle captured at `2026-04-21T20:16:26.493Z` on `0.4.159` /
`git 242f5056` changed the diagnosis again.

The worst completion outliers are no longer explained by VS Code UI, response egress, admission
queue, or shared `poll_ready()` wait:

- one authoritative trace spends `22797ms` in `completion_barrier_wait_ms` and `22796ms` in
  `same_file_ingress_token_wait_ms` while client pre-send stays at `1-1ms`;
- another trace spends `5574ms` in the same two fields while queue/scheduler/output waits stay
  near zero;
- in both traces the barrier owner is same-file `textDocument/didChange`.

Current code explains why this can still happen even after `refactor-47`:

- `refactor-47` intentionally made same-file ingress token publication truthful: only after
  current-revision handoff is actually registered;
- but the current implementation still performs that handoff registration and token publication
  inside `lsp_did_change`, after the document-sync request has already reached full handler code;
- so a later same-file completion can still wait seconds-scale for an earlier `didChange` merely
  because that `didChange` has not yet reached handler entry and registered its handoff.

So the remaining gap is narrower than generic transport starvation: same-file `didChange`
current-revision handoff itself is not yet isolated from delayed full-handler progression.

## What Changes
- Add a `bsl-intellisense-v2` contract that same-file `didChange` current-revision handoff for
  `(file_id, version)` progresses on a minimal ingress fast lane before full handler/background
  stages.
- Require later same-file completion to depend only on truthful handoff registration for the
  needed revision, not on delayed `didChange` handler entry, parse-snapshot scheduling,
  diagnostics scheduling, or other same-file auxiliary work.
- Preserve strict truthfulness: the same-file ingress token still cannot be published from a mere
  dispatcher event or barrier-owner record before handoff registration actually happened.
- Add representative mixed-load acceptance that fails if raw same-file `didChange` ingress is
  already observed before a later completion, but completion still spends seconds-scale
  `completion_barrier_wait_ms` / `same_file_ingress_token_wait_ms` waiting for handoff
  registration.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - representative runtime/perf/live acceptance around `examples/conf_big`
- Follow-up relationship:
  - builds on `refactor-47-completion-transport-runtime-isolation`
  - does not replace `refactor-current-revision-head-detached-snapshot`

## Non-Goals
- Do not publish same-file freshness from transport bookkeeping alone before current-revision
  handoff is actually registered.
- Do not reopen VS Code UI or response-egress investigation without new contradictory evidence.
- Do not require the long-term detached immutable snapshot architecture as a prerequisite.
- Do not optimize cold query-body / exact semantic latency outside this post-`didChange` handoff
  gap.
