## 1. Implementation
- [x] 1.1 Introduce a canonical ready-snapshot probe outcome model for didSave follow-up branch selection, covering both zero-budget and bounded-wait probes.
- [x] 1.2 Record branch-selection context in the save timeline, including same-version ready-snapshot task state and `shadow_state` availability.
- [x] 1.3 Extend the versioned diagnostics save timeline contract and VS Code incident-bundle projections with explicit older-version degradation semantics.

## 2. Validation
- [x] 2.1 Add regressions proving the save timeline distinguishes `ready_artifacts` success from `not_ready`, freshness-mismatch, timeout, and cancellation/supersession cases.
- [x] 2.2 Add projection tests proving older timeline payloads degrade explicitly as unavailable-by-design instead of silently dropping new fields.
- [x] 2.3 Run `openspec validate refactor-15-diagnostics-save-ready-snapshot-miss-attribution --strict --no-interactive`.

## 3. OpenSpec / Beads Sync
- [x] 3.1 Keep Beads epic `bsl-gradual-types-1rkq` and child `bsl-gradual-types-1rkq.1` aligned with the real implementation/validation status of this change.
- [x] 3.2 Keep `validation/epic-summary.md` aligned with the actual cross-change sequencing and Beads dependency graph for this epic.
