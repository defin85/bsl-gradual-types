# Change: prefer an in-flight same-version snapshot before shadow fallback on didSave

## Why

After the first two changes, the system should be able to answer two questions with evidence:

- why `ready_artifacts` was not chosen on `didSave`;
- why `didChange` did or did not materialize an exact-version parse snapshot in time.

If that evidence shows the common case is "same-version snapshot task exists but was not ready at
the zero-budget probe", then the current branch order on didSave becomes the next bottleneck:

- `ready_artifacts(0ms)`
- `shadow_state`
- `ready_artifacts(wait budget)`

In that order, `shadow_state` wins immediately and a near-ready exact snapshot cannot help the same
save cycle anymore.

## What Changes

- Require didSave heavy follow-up to prefer a bounded wait for an already-known in-flight
  same-version ready-snapshot task before consuming `shadow_state`.
- Keep the optimization conditional:
  - if there is no exact same-version task evidence, fall back immediately to truthful
    `shadow_state`/generic behavior;
  - if the task is stale, superseded, cancelled, or for another version, do not delay fallback.
- Preserve current fail-closed semantics and no-older-version publish guarantees.

## Sequence

This is the third change in the chain.

It should land only after the new observability from `refactor-15` and `refactor-16` confirms that
the dominant residual miss class is "exact same-version snapshot exists or is imminently materializing,
but current branch ordering still commits to `shadow_state` too early."

## Epic

This change is part of Beads epic `bsl-gradual-types-1rkq`
(`Epic: didSave snapshot reuse hardening follow-ups`).

Execution child for this step: `bsl-gradual-types-1rkq.3`.
Dependency order:
- blocked by `bsl-gradual-types-1rkq.2`
- final delivery step for this epic chain

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - didSave follow-up tests and incident-bundle validations

## Non-Goals

- Do not introduce a global wait before every `shadow_state` fallback.
- Do not mask missing/stale snapshots with speculative reuse.
- Do not fix didChange parse-snapshot materialization in this change.
