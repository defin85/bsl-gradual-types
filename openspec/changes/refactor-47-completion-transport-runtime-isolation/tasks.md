## 1. Contract

- [ ] 1.1 Define the transport-runtime contract for completion admission after the
      `2026-04-21T09:41:02Z` incident bundle:
      the diagnosis is split into `client/write -> adapter_read` and `adapter_read -> dispatch`,
      reader/scheduler/output progression is task-isolated where needed, and same-file document
      ingress ownership is explicit instead of implicit FIFO coupling.
- [ ] 1.2 Define the additive completion timeline / incident-bundle contract for truthful
      decomposition of reader wait, queue residence, shared-readiness wait, barrier ownership,
      same-file token wait, and residual post-ready delay, including anti-bucket-shift acceptance.
- [ ] 1.3 Define the exact publication point for same-file ingress tokens:
      only after current-revision handoff is registered for that file/version, not at
      dispatcher-event emission time.

## 2. Implementation

- [ ] 2.1 Export the new completion pre-dispatch decomposition fields and update incident-bundle /
      timeline projections without inventing client-side blame:
      reader wait, spillover/backpressure state, queue residence, shared `poll_ready()` wait,
      completion barrier wait with owner identity, doc-sync first-poll identity/duration,
      same-file token wait/source, and residual post-ready delay.
- [ ] 2.2 Introduce explicit same-file document ingress ownership/token publication that preserves
      raw document-sync ordering, latest-wins semantics, and cancellation/supersession invariants,
      and publishes the token only after current-revision handoff registration.
- [ ] 2.3 Rework completion admission so first response depends on the relevant same-file ingress
      token instead of unrelated same-priority FIFO residence, while preserving bounded spillover
      and exactly-once terminal semantics.
- [ ] 2.4 Split the transport runtime into independently progressing reader, single-owner
      scheduler, output writer, and completion handoff tasks or an equivalent starvation-safe
      topology without regressing single-owner `poll_ready()/call()` semantics, and only count the
      change as a latency fix if the new buckets show real bounded waits.

## 3. Regressions and evidence

- [ ] 3.1 Add backend regressions proving a stalled scheduler branch cannot stop transport reader
      progress, late cancel classification, or ready response output progression.
- [ ] 3.2 Add reader-backpressure regressions proving local spillover wait before `adapter_read`
      is explicitly attributed and cannot silently masquerade as client-side ingress.
- [ ] 3.3 Add same-file transport-path regressions proving `didChange`/`didSave` ingress ownership
      reaches a later completion ahead of unrelated queued work once the relevant file token is
      available after current-revision handoff registration.
- [ ] 3.4 Refresh representative live/perf evidence on `examples/conf_big` showing that the new
      dominant seams are bounded, completion stays fail-closed/correct under mixed load, and no
      seconds-scale wait merely moved from the old umbrella field into a new bucket.
- [ ] 3.5 Attach one correlation slice for the worst representative outlier:
      active same-file `didChange/current-revision` pressure, barrier owner, required token
      version, and current published token version/source.

## 4. Validation

- [ ] 4.1 Run targeted backend/runtime/transport/perf/incident-bundle regressions for task
      isolation, reader backpressure attribution, same-file ingress ownership, and truthful
      pre-dispatch decomposition.
- [ ] 4.2 Run `openspec validate refactor-47-completion-transport-runtime-isolation --strict --no-interactive`.
