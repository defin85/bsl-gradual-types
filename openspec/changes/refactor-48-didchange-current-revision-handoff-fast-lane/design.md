## Context
The new incident bundle `bsl-observability-incident-2026-04-21T20-16-26Z` shows that the dominant
completion residual moved again.

The representative outliers are not UI-first and not generic queue starvation:

- `completion-trace-2` spends `22797ms` in `completion_barrier_wait_ms` and `22796ms` in
  `same_file_ingress_token_wait_ms`;
- `completion-trace-5` spends `5574ms` in the same buckets;
- both traces attribute the owner to same-file `textDocument/didChange`;
- client pre-send, admission queue, shared `poll_ready()`, handler prelude, and output waits are
  all negligible in those traces.

Current code already satisfies the truthful part of `refactor-47`, but not the latency part needed
by this new bundle:

- `publish_same_file_ingress_token_v2(...)` still runs inside `lsp_did_change`;
- the same handler also owns shadow-state update and `analysis_v2.apply_changes_interactive(...)`
  for the new revision;
- therefore the current-revision handoff is only registered once that `didChange` reaches full
  handler execution, even if transport already observed the request much earlier.

This change is therefore not about detached snapshots, not about broader UI/client blame, and not
about redoing the transport split from `refactor-47`. It is about moving the minimal same-file
handoff progression point ahead of delayed full-handler work while keeping the truthful
"token only after handoff" rule intact.

## Goals
- Make same-file `didChange` current-revision handoff registration a fast-lane progression point.
- Keep later same-file completion waiting on truthful handoff registration, not on full
  `didChange` handler entry.
- Preserve latest-wins, out-of-order safety, and fail-closed semantics.
- Add representative evidence that fails if this residual comes back on `examples/conf_big`.

## Non-Goals
- Do not publish same-file tokens before current-revision handoff is actually registered.
- Do not widen the scope into detached immutable read models.
- Do not use client-side heuristics instead of backend remediation.
- Do not optimize unrelated cold semantic/query-body cost in the same change.

## Decisions

### 1. Introduce a minimal same-file didChange handoff fast lane
The server will gain a minimal progression path for `textDocument/didChange` whose only job is to
make the new current revision observable for later same-file interactive work.

That minimal path owns:

- deriving the canonical updated text for the accepted `didChange`;
- updating `latest_received` and same-file shadow state for that revision;
- registering the corresponding current-revision `SetFile` handoff in the runtime writer path;
- publishing the same-file ingress token only after that handoff is registered.

This fast lane does not own parse-snapshot scheduling, diagnostics scheduling, exact/type-index
precompute, or other auxiliary work.

### 2. Full didChange handler work becomes downstream of the registered handoff
The existing `lsp_did_change` path still owns:

- parse-snapshot attribution and scheduling;
- parser-base recovery decisions;
- completion-head reuse / alias decisions;
- diagnostics scheduling and related observability.

But that downstream work must operate on a revision whose current-revision handoff may already be
registered. Implementation may extract a shared helper or another equivalent structure, but the
contract is:

- no duplicate `SetFile` side effects for the same accepted revision;
- no stale overwrite of a newer same-file revision;
- no fake readiness stronger than the handoff that actually occurred.

### 3. Latest-wins semantics remain authoritative
The new fast lane must preserve the same-file safety invariants already expected by current code:

- out-of-order older `didChange` revisions are dropped instead of overwriting newer shadow state;
- a superseded older revision cannot publish a misleading same-file token after a newer revision
  already became authoritative;
- later same-file completion waits only on the latest required revision.

This change is not allowed to treat "dispatcher event seen" as equal to "handoff registered".
Truthful publication remains anchored to the real handoff point.

### 4. Acceptance is driven by representative same-file mixed load
Representative live/perf evidence must exercise the exact failure mode from the new bundle:

- same-file `didChange` reaches ingress first;
- later same-file completion arrives while unrelated work may also exist;
- the measured completion trace still fails if seconds-scale time remains in
  `completion_barrier_wait_ms` or `same_file_ingress_token_wait_ms`.

The checked-in evidence must keep at least one worst-outlier correlation slice that preserves:

- the barrier owner method and version;
- the completion-required same-file revision;
- when the handoff/token became observable for that revision.

## Alternatives Considered

### Publish the token from dispatcher/barrier bookkeeping
Rejected. `refactor-47` explicitly moved away from this because transport-visible bookkeeping does
not prove that current-revision handoff is actually registered.

### Jump directly to detached immutable current-revision head snapshot
Rejected for this change. That remains a larger architectural track in
`refactor-current-revision-head-detached-snapshot`, while the current residual is narrower and
needs an immediate remediation on the existing runtime contract.

### Accept the current state and only keep better traces
Rejected. The bundle already proves the problem clearly enough; this is not merely an
observability gap.

## Risks / Trade-offs
- Risk: splitting didChange responsibilities introduces duplicate text-application logic.
  - Mitigation: centralize the shared "accepted revision -> shadow update -> handoff register"
    primitive instead of re-implementing it twice.
- Risk: fast-lane registration races with downstream handler stages.
  - Mitigation: keep same-file latest-wins ownership explicit and make downstream work consume the
    already-authoritative revision instead of re-registering it.
- Risk: representative acceptance overfits `conf_big`.
  - Mitigation: keep `conf_big` as the large-module reproducer, but express the requirement as a
    class of same-file post-edit behavior rather than a fixture-specific hack.

## Open Questions
- Should the fast lane live directly in transport ingress, or in a dedicated document-sync ingress
  queue immediately downstream of transport decode? Either is acceptable if the same contract and
  evidence hold.
