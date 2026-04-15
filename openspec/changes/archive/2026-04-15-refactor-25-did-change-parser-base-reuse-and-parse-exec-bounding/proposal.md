# Change: reduce exact ready-snapshot misses by restoring parser-base reuse and bounding parse-exec waste

## Why

The latest `conf_big` incident bundles are no longer ambiguous:

- ranged `didChange` repeatedly falls back to `full` parse from `shadow_state` with
  `fallback_reason=stale_parser_base`;
- the bounded root cause is `ready_snapshot_lags_shadow_state`;
- exact ready-snapshot follow-up then times out while the same-version producer is still in
  `parse_exec`;
- the temporary `didSave` relief valve from `refactor-24` engages truthfully, but still times out.

That means `refactor-22` through `refactor-24` did their job: they explained the failure and added
an operational valve. They did not remove the runtime root cause.

The next fix should therefore target the two remaining bottlenecks directly:

1. restore a truthful bounded reuse/prime path so ranged `didChange` does not immediately pay for a
   full parse whenever ready snapshots merely lag shadow state;
2. reduce obsolete `parse_exec` waste so same-file retarget/coalescing can abort expensive work
   earlier instead of discovering obsolescence only after most of the parse cost has already been
   paid.

## What Changes

- Require a bounded parser-base recovery path for ranged `didChange` when the current miss class is
  `ready_snapshot_lags_shadow_state`, instead of treating that state as an immediate full-parse
  fate.
- Require exact ready-snapshot parse execution to observe newer same-file retarget/cancel signals
  inside the expensive parse/build path, so obsolete work can stop during `parse_exec` rather than
  only before materialization.
- Require regressions and repo-local evidence that show whether these fixes move the real incident
  back toward `ready_artifacts` rather than only making the fallback more observable.

## Sequence

This change intentionally follows:

- `refactor-22-did-change-parser-base-root-cause-attribution`
- `refactor-23-ready-snapshot-materialization-phase-attribution`
- `refactor-24-diagnostics-save-followup-budget-valve`

Those changes established truthful attribution. This one is the first root-cause runtime fix that
is allowed to use that attribution to change behavior.

## Epic

This change is tracked by Beads epic `bsl-gradual-types-qtm3`
(`OpenSpec refactor-25: parser-base recovery and parse-exec waste bounding`).

Execution children for this step:

- `bsl-gradual-types-qtm3.1` - bounded parser-base recovery for
  `ready_snapshot_lags_shadow_state`
- `bsl-gradual-types-qtm3.2` - during-parse retarget/cancel bounding for obsolete exact work
- `bsl-gradual-types-qtm3.3` - regressions and `conf_big` live evidence
- `bsl-gradual-types-qtm3.4` - targeted validation and strict OpenSpec validation

Dependency graph:

- `bsl-gradual-types-qtm3.1` and `bsl-gradual-types-qtm3.2` may proceed independently;
- `bsl-gradual-types-qtm3.3` depends on both implementation branches;
- `bsl-gradual-types-qtm3.4` depends on `bsl-gradual-types-qtm3.3`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - ready-snapshot lifecycle / observability metrics / live evidence around `conf_big`

## Non-Goals

- Do not widen the base `didSave` wait budget or the temporary relief valve as the primary fix.
- Do not relax exactness requirements for same-version ready artifacts.
- Do not replace truthful fallback-to-`shadow_state` behavior when the new bounded reuse/cancel
  path still cannot produce an exact ready snapshot in time.
