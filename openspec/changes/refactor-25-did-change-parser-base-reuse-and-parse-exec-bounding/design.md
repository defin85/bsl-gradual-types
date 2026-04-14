## Context

`refactor-22` through `refactor-24` narrowed the incident profile to one backend chain:

- ranged `didChange` falls into `stale_parser_base`;
- the bounded miss class is `ready_snapshot_lags_shadow_state`;
- exact same-version ready-snapshot work then spends most of its wall time in `parse_exec`;
- newer same-file revisions often retarget the worker only after nearly all parse cost has already
  been paid.

The current system is therefore truthful but still too expensive under `conf_big` churn.

## Goals / Non-Goals

- Goals:
  - recover a truthful bounded parser-base reuse path when ready snapshots merely lag shadow state;
  - stop obsolete exact parse work earlier during `parse_exec`;
  - keep fallback and observability truthful when recovery is impossible.
- Non-Goals:
  - no UI/client-side changes;
  - no generic widening of `didSave` wait budgets;
  - no weakening of exact same-version semantics.

## Decisions

### Decision: `ready_snapshot_lags_shadow_state` should no longer imply immediate full-parse fate

The miss class added in `refactor-22` tells us that the failure is often not "bad edits" or "no
base exists anywhere". It is specifically "the authoritative ready snapshot has not caught up to
the already authoritative shadow text".

That case should first attempt a bounded recovery/prime path against the current same-version
document state. Only after that bounded attempt fails should the runtime fall back to the existing
truthful full-parse path.

### Decision: abortability must move into the expensive parse/build path

Current retarget checks before parse and after parse are not enough. If the expensive work is a
multi-step blocking build, detecting obsolescence only after `BuildParseSnapshotOutcome::Ready`
still wastes most of the latency budget.

This change should therefore introduce at least one additional cancellation/retarget observation
point inside the expensive parse/build path, and expose a bounded lifecycle reason for "aborted
during parse execution".

### Decision: keep behavior fail-closed when recovery or mid-parse abort proof is absent

The runtime must stay exactness-first. If the new recovery path cannot prove a matching parser base
or the parse path cannot safely abort yet, the system should preserve the current truthful fallback
instead of fabricating ready-artifact success.

## Alternatives Considered

### Raise the `didSave` wait budget again

Rejected. The current bundles already show that this would only stretch latency around the same
root cause.

### Accept more `shadow_state` semantic work as the normal answer

Rejected. That makes the regression operationally quieter, not fixed.

### Re-run every ranged `didChange` as unconditional full parse

Rejected. It contradicts the existing incremental design and would regress typing-load behavior on
large modules.

## Risks / Trade-offs

- Parser-base recovery can become misleading if it silently uses a non-matching base; validation
  must stay exactness-first.
- Mid-parse cancellation can be hard to place if underlying parse/build steps are too monolithic;
  the design should prefer bounded checkpoints over speculative complexity.
- Real `conf_big` improvement may still depend on both fixes landing together; partial rollout can
  leave observability improved while latency remains high.

## Migration Plan

1. Add bounded parser-base recovery for the `ready_snapshot_lags_shadow_state` miss class.
2. Add parse-exec cancellation/retarget checkpoints and lifecycle attribution for during-parse
   aborts.
3. Re-run the repo-local mixed-load profile and record whether same-version `didSave` follow-up
   returns to `ready_artifacts`.
