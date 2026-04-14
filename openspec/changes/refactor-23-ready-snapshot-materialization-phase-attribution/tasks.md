## 1. Phase attribution contract

- [x] 1.1 Export bounded phase-level latency attribution for exact ready-snapshot production,
      including at least parse execution, post-parse/pre-materialization, and ready-install.
- [x] 1.2 Export a truthful producer-phase-at-timeout signal so `didSave` bundles can state where
      the exact worker was when bounded wait expired.
- [x] 1.3 Keep documentSymbol / outline side-work attributable as a separate non-readiness phase.

## 2. Regressions and evidence

- [x] 2.1 Add backend regressions covering producer timeout in parsing vs post-parse windows.
- [x] 2.2 Add evidence that symbol/outline side-work does not inflate exact ready-install phase.
- [x] 2.3 Capture repo-local live evidence on `conf_big` showing dominant exact-path phase.

## 3. Validation

- [x] 3.1 Run targeted backend tests for new phase metrics, timeout attribution, and bundle export.
- [x] 3.2 Run `openspec validate refactor-23-ready-snapshot-materialization-phase-attribution --strict --no-interactive`.

## 4. OpenSpec / Beads Sync

- [x] 4.1 Keep Beads epic `bsl-gradual-types-wikt` and child `bsl-gradual-types-wikt.2`
      aligned with the current plan/status of this change.
