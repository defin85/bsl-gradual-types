## 1. Temporary valve contract

- [x] 1.1 Add a strictly bounded temporary relief window for `didSave` heavy follow-up only when
      runtime can prove it is waiting on an exact still-current producer.
- [x] 1.2 Gate the valve off for queue/apply-lag cases, coalesced-away producers, and other
      non-exact fallback paths.
- [x] 1.3 Export explicit observability for valve engaged / skipped / ineffective outcomes.

## 2. Regressions and evidence

- [x] 2.1 Add backend regressions proving the valve helps only the exact-path late-materialization
      case and does not mask queue/apply-lag or retargeted-away cases.
- [x] 2.2 Capture repo-local evidence comparing base-budget vs temporary-valve outcomes on the same
      real save cycle profile.
- [x] 2.3 Document a sunset condition for removing or disabling the valve once the root cause is
      fixed.

## 3. Validation

- [x] 3.1 Run targeted backend tests for valve gating, timeout attribution, and bundle export.
- [x] 3.2 Run `openspec validate refactor-24-diagnostics-save-followup-budget-valve --strict --no-interactive`.

## 4. OpenSpec / Beads Sync

- [x] 4.1 Keep Beads epic `bsl-gradual-types-wikt` and child `bsl-gradual-types-wikt.3`
      aligned with the current plan/status of this change.
