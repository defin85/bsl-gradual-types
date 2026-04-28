## 1. Contract

- [x] 1.1 Add `bsl-intellisense-v2` requirements for lane-aware runtime
      saturation taxonomy and zero invalid saturation contract violations.
- [x] 1.2 Record the new bundle evidence:
      `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric=180`
      on commit `8a406809`.
- [x] 1.3 State explicitly that this change does not reopen `refactor-56` and
      does not implement the full observability pipeline rewrite.

## 2. Implementation

- [x] 2.1 Remove or reroute generic saturation emissions for
      `waiters_did_save_followup` and `permits_did_save_followup` so they no
      longer enter the generic `SaturationGauge` allowlist path.
- [x] 2.2 Preserve dedicated runtime-lane saturation gauges for
      `lane=did_save_followup` with bounded metrics `quota`, `active_slots`, and
      `queue_depth`; source them from the didSave-follow-up admission state
      (`record_did_save_followup_lane_saturation_v2` or equivalent), not from
      `CpuBudgetSaturationSnapshot`.
- [x] 2.3 Preserve existing `queue_depth_total` compatibility semantics
      (`interactive_waiters + background_waiters`); do not silently fold
      didSave-follow-up lane queue depth into the generic gauge.
- [x] 2.4 Add or tighten a `BasicObservability` projection test so every accepted
      generic saturation metric has both drilldown and legacy projection, while
      lane-specific saturation stays on the lane family.
- [x] 2.5 Add a negative test proving `waiters_did_save_followup` and
      `permits_did_save_followup` still fail if attempted through generic
      `SaturationGauge`, and do not export generic drilldown/legacy keys.
- [x] 2.6 Add a policy/backend test proving generic gauges and dedicated
      didSave-follow-up lane gauges are both exported without contract
      violations.
- [x] 2.7 Ensure incident-bundle or metrics-summary consumers surface
      `observability_contract_violation_total` clearly enough for regression
      review.

## 3. Validation

- [x] 3.1 Run focused runtime observability tests, including the new saturation
      contract coverage.
- [x] 3.2 Run relevant backend/LSP metrics export coverage if touched.
- [x] 3.3 Capture a fresh representative metrics snapshot or incident bundle and
      verify:
      - `intellisense_v2_observability_contract_violation_total` is absent or
        `0`;
      - `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric`
        is absent or `0`;
      - `intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_quota`
        is present;
      - didSave-follow-up queue/active-slot visibility is preserved.
- [x] 3.4 Run `cargo check --workspace --all-targets`.
- [x] 3.5 Run `cargo clippy --workspace --all-targets -- -D warnings` if
      production Rust changes are made.
- [x] 3.6 Run
      `openspec validate refactor-57-runtime-saturation-contract-completeness --strict --no-interactive`.
