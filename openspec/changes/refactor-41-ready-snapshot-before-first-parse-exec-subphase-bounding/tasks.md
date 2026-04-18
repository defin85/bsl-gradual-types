## 1. Contract

- [ ] 1.1 Define the representative save-follow-up contract for still-current exact producers that
      currently time out at `before_first_parse_exec_subphase` yet later publish through
      `ready_artifacts`.
- [ ] 1.2 Preserve truthful differentiation between `continued_still_current`,
      exhausted-continuation, supersession, and cancellation outcomes without widening bounded wait
      or relief-valve budgets.

## 2. Implementation

- [ ] 2.1 Identify and bound the dominant pre-first-subphase `parse_exec` residence on the
      representative same-file `didChange` / `didSave` path instead of assuming a later checkpoint
      is still dominant.
- [ ] 2.2 Rework the save-critical exact producer so it either reaches a bounded first in-parse
      progress point or materially reduces the opaque entry span before the representative
      follow-up spends most of its wall-clock time waiting.
- [ ] 2.3 Preserve exact same-version semantics, latest-wins supersession, and truthful
      continuation/fallback evidence while the early `parse_exec` path is reworked.
- [ ] 2.4 If refreshed truthful evidence after 2.1-2.3 still shows a later residual dominating,
      bound that next checkpoint with direct proof instead of carrying forward the old assumption
      set.

## 3. Regressions and evidence

- [ ] 3.1 Add targeted backend/runtime regressions for still-current
      `before_first_parse_exec_subphase` continuation, truthful supersession, and non-stale exact
      publish.
- [ ] 3.2 Refresh representative incident/live evidence against the `2026-04-18T18:52:50Z`
      baseline, including `followup_publish_elapsed_ms`,
      `followup_ready_snapshot_parse_exec_ms`, ready-snapshot materialization latency, and
      terminal path incidence.

## 4. Validation

- [ ] 4.1 Run targeted parser/runtime/backend tests for the new early-`parse_exec` contract.
- [ ] 4.2 Run `openspec validate
      refactor-41-ready-snapshot-before-first-parse-exec-subphase-bounding --strict
      --no-interactive`.
