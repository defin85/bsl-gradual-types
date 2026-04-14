## 1. Save-critical exact `core_parse_build`

- [x] 1.1 Bound the exact same-version `core_parse_build` path so the producer can materialize
      publishable exact ready artifacts without waiting for secondary core-build work that is not
      required for the first exact follow-up publish.
- [x] 1.2 Preserve fail-closed exactness and supersession semantics when a newer same-file target
      arrives or when the bounded core-build path still cannot prove current exact artifacts.

## 2. Core-build checkpoints and attribution

- [x] 2.1 Split `core_parse_build` into bounded observable checkpoints or subphases that can
      identify which exact core-build slice dominates the timeout path after `refactor-27`.
- [x] 2.2 Export the finer core-build attribution through diagnostics save timeline / incident
      bundle surfaces without regressing the phase- and subphase-level truthfulness added by
      `refactor-23` through `refactor-27`.

## 3. Regressions and live evidence

- [x] 3.1 Add backend regressions for:
      save-critical exact `core_parse_build` finishing without secondary core-build work on the
      critical path,
      truthful residual attribution when exact `core_parse_build` still times out,
      and non-regression of supersession / retarget behavior inside the new core-build checkpoints.
- [x] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which exact
      `core_parse_build` slice remains dominant after the fix.

## 4. Validation

- [x] 4.1 Run targeted backend tests covering save-critical exact `core_parse_build`, timeout
      checkpoint attribution, and the relevant `didSave` follow-up path.
- [x] 4.2 Run `openspec validate refactor-28-diagnostics-save-exact-core-parse-build-bounding --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [x] 5.1 Keep Beads epic `bsl-gradual-types-zb0m` and children
      `bsl-gradual-types-zb0m.1` through `bsl-gradual-types-zb0m.4` aligned with the intended
      implementation status and dependency graph of this change.
