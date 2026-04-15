## 1. Save-critical exact `ready_snapshot_assembly`

- [x] 1.1 Bound the exact same-version `ready_snapshot_assembly` path so the producer can
      materialize publishable exact ready artifacts without waiting for secondary assembly work
      that is not required for the first exact follow-up publish.
- [x] 1.2 Preserve fail-closed exactness and supersession semantics when a newer same-file target
      arrives or when the bounded assembly path still cannot prove current exact artifacts.

## 2. Assembly checkpoints and attribution

- [x] 2.1 Split `ready_snapshot_assembly` into bounded observable checkpoints or subphases that can
      identify which exact assembly slice dominates the timeout path after `refactor-28`.
- [x] 2.2 Export the finer assembly attribution through diagnostics save timeline / incident bundle
      surfaces without regressing the phase-, subphase-, and core-build-checkpoint truthfulness
      added by `refactor-23` through `refactor-28`.

## 3. Regressions and live evidence

- [x] 3.1 Add backend regressions for:
      save-critical exact `ready_snapshot_assembly` finishing without secondary assembly work on
      the critical path,
      truthful residual attribution when exact `ready_snapshot_assembly` still times out,
      and non-regression of supersession / retarget behavior inside the new assembly checkpoints.
- [x] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which exact
      `ready_snapshot_assembly` slice remains dominant after the fix.

## 4. Validation

- [x] 4.1 Run targeted backend tests covering save-critical exact `ready_snapshot_assembly`,
      timeout checkpoint attribution, and the relevant `didSave` follow-up path.
- [x] 4.2 Run `openspec validate refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [x] 5.1 Keep Beads epic `bsl-gradual-types-z7od` and children
      `bsl-gradual-types-z7od.1` through `bsl-gradual-types-z7od.4` aligned with the intended
      implementation status and dependency graph of this change.
