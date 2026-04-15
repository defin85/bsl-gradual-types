## 1. Save-critical exact `program_conversion`

- [x] 1.1 Bound the exact same-version `program_conversion` path so the producer can materialize
      publishable exact ready artifacts without waiting for secondary conversion or packaging work
      that is not required for the first exact follow-up publish.
- [x] 1.2 Preserve fail-closed exactness and supersession semantics when a newer same-file target
      arrives or when the bounded conversion path still cannot prove current exact artifacts.

## 2. Program-conversion checkpoints and attribution

- [x] 2.1 Split `program_conversion` into bounded observable checkpoints or subphases that can
      identify which exact conversion slice dominates the timeout path after `refactor-29`.
- [x] 2.2 Export the finer conversion attribution through diagnostics save timeline / incident
      bundle surfaces without regressing the phase-, subphase-, core-build-, and
      assembly-checkpoint truthfulness added by `refactor-23` through `refactor-29`.

## 3. Regressions and live evidence

- [x] 3.1 Add backend regressions for:
      save-critical exact `program_conversion` finishing without secondary conversion work on the
      critical path,
      truthful residual attribution when exact `program_conversion` still times out,
      and non-regression of supersession / retarget behavior inside the new conversion checkpoints.
- [x] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which exact
      `program_conversion` slice remains dominant after the fix.

## 4. Validation

- [x] 4.1 Run targeted backend tests covering save-critical exact `program_conversion`, timeout
      checkpoint attribution, and the relevant `didSave` follow-up path.
- [x] 4.2 Run VS Code diagnostics-save request / incident-bundle tests if the timeline contract or
      bundle rendering changes.
- [x] 4.3 Run `openspec validate refactor-30-diagnostics-save-exact-ready-snapshot-program-conversion-bounding --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [x] 5.1 Keep Beads epic `bsl-gradual-types-ptc7` and children
      `bsl-gradual-types-ptc7.1` through `bsl-gradual-types-ptc7.4` aligned with the intended
      implementation status and dependency graph of this change.
