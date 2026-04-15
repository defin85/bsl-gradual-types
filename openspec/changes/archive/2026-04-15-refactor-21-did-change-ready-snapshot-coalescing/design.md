## Context

After `refactor-20` and the recent `stale_parser_base` fix, the runtime now reports the right
cause for the expensive ranged `didChange` path. The remaining issue is no longer attribution but
same-file work shaping:

- exact fallback cause is visible as `stale_parser_base`;
- same-file `didChange` still starts many ready-snapshot workers and supersedes most of them;
- `didSave` heavy follow-up still times out and falls back to `shadow_state`.

This points to a scheduling problem: the system still behaves too much like "one worker per
revision" for same-file bursts, even though only the newest exact revision matters.

## Goals / Non-Goals

- Goals:
  - Bound same-file `didChange` ready-snapshot churn through latest-wins coalescing.
  - Preserve exact current-revision semantics for ready artifacts and `didSave` follow-up.
  - Make incident bundles show whether the producer really timed out or was coalesced away earlier.
- Non-Goals:
  - No larger `didSave` wait budget.
  - No duplicate `didSave` snapshot worker.
  - No new parser fallback taxonomy beyond the existing truthful reasons.
  - No binary build-identity fix in this change.

## Decisions

### Decision: use a file-scoped coalesced ready-snapshot producer for `didChange`

For same-file `didChange` bursts, the server SHOULD keep at most one file-scoped background producer
responsible for the latest exact ready-snapshot target.

Each new same-file revision updates the producer target `(requested_version, text_hash, text)` and
notifies the producer instead of spawning another background worker immediately.

The producer loop remains latest-wins:

1. absorb coalesced newer targets during debounce;
2. capture the latest target before blocking parse starts;
3. if a newer target arrives before materialization/install, skip stale materialization and retarget
   to the newest exact revision;
4. materialize only if the built snapshot still matches the latest exact target.

This preserves current correctness while reducing obsolete worker starts and stale installs.

### Decision: `didSave` waits only on an exact still-current producer

`didSave` heavy follow-up should continue using bounded wait, but only when the runtime can prove
that the file-scoped producer is still exact for `(file_id, requested_version, text_hash)`.

If the producer has already been retargeted to a newer same-file revision, `didSave` must not burn
bounded wait on a no-longer-exact path and should fall back immediately to truthful `shadow_state`
or the generic path.

### Decision: observability must distinguish coalescing from timeout

The current bundle can show `stale_parser_base`, but still compresses too much scheduling context
into `started/superseded/materialized`.

This change should expose low-cardinality producer lifecycle outcomes such as:

- exact producer retargeted before parse;
- stale built snapshot skipped before materialization because a newer target already existed;
- exact same-version producer promoted by `didSave`;
- bounded wait still timed out and fell back to `shadow_state`.

The goal is operational clarity: distinguish "we never should have waited for that revision" from
"we waited for the right revision and still lost to the budget".

## Alternatives Considered

### Keep spawn-per-revision workers and only raise wait budgets

Rejected. The bundles already show too much obsolete same-file work. Larger waits would only hide
the churn and add more latency to `didSave`.

### Spawn a dedicated `didSave` exact worker

Rejected. That duplicates parse work for the same exact revision and weakens the latest-wins model.

## Risks / Trade-offs

- A file-scoped producer is more stateful than spawn-per-revision scheduling and needs careful
  ownership around debounce, parse, and materialization checkpoints.
- A long full parse for the latest exact revision can still dominate latency; coalescing reduces
  wasted intermediate work but does not make the final exact parse free.
- Observability must stay low-cardinality; this change must not export per-revision free-form
  lifecycle strings.

## Migration Plan

1. Add producer state and exact-target matching for same-file `didChange`.
2. Route `didSave` bounded wait through that exact-target state.
3. Add bundle-visible coalescing lifecycle metrics.
4. Re-run the same repo-local `conf_big` save-cycle evidence and compare worker churn plus
   `ready_artifacts` vs `shadow_state` outcomes.

## Open Questions

- Whether `bsl.getCurrentContext` should also reuse the same coalesced producer in the same change
  or remain a later follow-up.
- Whether snapshot-status live notifications should surface coalesced-retarget hints directly, or
  whether incident-bundle/export-only observability is sufficient.
