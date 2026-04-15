## 1. Implementation
- [x] 1.1 Teach didSave heavy follow-up branch selection to distinguish an in-flight exact same-version ready-snapshot task from all other miss classes.
- [x] 1.2 Reorder the didSave branch selection only for that exact-task case so the runtime tries bounded ready-artifact waiting before `shadow_state`.
- [x] 1.3 Preserve current fail-closed fallback behavior for absent, stale, superseded, cancelled, or other-version task states.

## 2. Validation
- [x] 2.1 Add regressions proving an in-flight same-version snapshot can win the same save cycle without publishing stale diagnostics.
- [x] 2.2 Add regressions proving the runtime still falls back immediately to `shadow_state` when no exact same-version task exists.
- [x] 2.3 Capture representative evidence showing the reordered branch reduces `shadow_state+salsa` follow-up cycles only for the exact-task-in-flight case.
- [x] 2.4 Run `openspec validate refactor-17-diagnostics-save-inflight-snapshot-preference --strict --no-interactive`.

## 3. OpenSpec / Beads Sync
- [x] 3.1 Keep Beads epic `bsl-gradual-types-1rkq` and child `bsl-gradual-types-1rkq.3` aligned with the real implementation/validation status of this change.
