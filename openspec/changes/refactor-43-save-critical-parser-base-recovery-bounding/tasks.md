## 1. Contract

- [ ] 1.1 Define the save-critical `parser_base_recovery` contract for same-version exact
      producers that remain still-current while `didSave` heavy follow-up is waiting.
- [ ] 1.2 Define the derived incident-bundle fidelity contract for preserving diagnostics-save
      timeout-leaf facts from authoritative raw traces.

## 2. Implementation

- [ ] 2.1 Identify why the current same-version exact producer still spends representative
      steady-state latency inside `parser_base_recovery` despite the existing parser-base recovery
      and early-checkpoint refactors.
- [ ] 2.2 Rework save-critical parser-base recovery so the representative `conf_big` path no longer
      regresses into multi-second `didChange` materialization lag and `shadow_state` follow-up
      fallback solely because recovery monopolized the exact path.
- [ ] 2.3 Preserve exact same-version semantics, latest-wins supersession, truthful fallback, and
      bounded fail-closed behavior for downstream exact consumers such as `bsl.getCurrentContext`
      when parser-base recovery proof genuinely cannot succeed.
- [ ] 2.4 Repair incident-bundle derived projection so
      `followup_ready_snapshot_timeout_leaf` and its elapsed fact survive from authoritative
      diagnostics-save traces into `incident.json` and `summary.md`.

## 3. Regressions and Evidence

- [ ] 3.1 Add targeted backend/runtime regressions for still-current save-critical
      `parser_base_recovery`, exhausted recovery proof, and non-stale exact publish.
- [ ] 3.2 Add direct incident-bundle regressions proving that derived request summaries preserve
      authoritative diagnostics-save timeout-leaf fields when the backend exports them.
- [ ] 3.3 Refresh representative live evidence against the `2026-04-19T14:34:41.582Z` incident
      bundle, including `didChange` ready-snapshot materialization, `didSave` terminal path,
      parser-base recovery dominance, and same-family current-context parse source distribution.

## 4. Validation

- [ ] 4.1 Run targeted backend/runtime/extension tests for save-critical parser-base recovery and
      incident-bundle timeout-leaf fidelity.
- [ ] 4.2 Run `openspec validate refactor-43-save-critical-parser-base-recovery-bounding --strict
      --no-interactive`.
