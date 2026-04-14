## 1. Save-critical exact `parse_exec`

- [ ] 1.1 Add a save-critical exact parse path for same-version `didSave` follow-up so the
      producer can materialize exact ready artifacts without paying for optional in-parse work
      that is not required for the publishable ready snapshot.
- [ ] 1.2 Preserve fail-closed exactness and supersession semantics when a newer same-file target
      arrives or when the save-critical path still cannot prove current exact artifacts.

## 2. Bounded in-parse checkpoints and attribution

- [ ] 2.1 Split exact `parse_exec` into bounded observable subphases or checkpoints that can
      surface which exact in-parse slice dominates the timeout path.
- [ ] 2.2 Export the new subphase attribution through diagnostics save timeline / incident bundle
      surfaces without regressing the existing phase- and blocker-level truthfulness from
      `refactor-23` through `refactor-26`.

## 3. Regressions and live evidence

- [ ] 3.1 Add backend regressions for:
      save-critical exact `parse_exec` finishing without deferred optional work on the critical
      path,
      truthful subphase attribution when exact `parse_exec` still times out,
      and non-regression of supersession / retarget behavior.
- [ ] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which exact
      `parse_exec` subphase remains dominant after the fix.

## 4. Validation

- [ ] 4.1 Run targeted backend tests covering save-critical exact `parse_exec`, timeout subphase
      attribution, and the relevant `didSave` follow-up path.
- [ ] 4.2 Run `openspec validate refactor-27-diagnostics-save-exact-parse-exec-bounding --strict --no-interactive`.
