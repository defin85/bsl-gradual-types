## ADDED Requirements

### Requirement: Runtime saturation taxonomy MUST be lane-aware and contract-complete (MUST)

The system MUST keep generic runtime saturation gauges and dedicated runtime-lane
saturation gauges as separate bounded taxonomy surfaces.

The generic saturation-gauge surface MUST:

- represent only global runtime budget facts such as interactive/background
  waiters, interactive/background/shared permits, and total queue depth;
- keep `queue_depth_total` as the existing compatibility view over generic
  interactive/background runtime waiters unless a separate change explicitly
  redefines that gauge;
- accept only low-cardinality `saturation_metric` values that are explicitly
  allowlisted;
- provide deterministic drilldown and legacy projection for every accepted
  generic saturation value;
- fail tests before merge if a new generic value lacks allowlist or projection
  coverage.

Dedicated lane saturation MUST:

- expose a stable `did_save_followup` value through a bounded runtime-lane
  surface, or a semantically equivalent first-class lane family visible in the
  exported metric name or bounded dimensions;
- preserve `queue_depth`, `quota`, and `active_slots` visibility for that lane;
- source those lane facts from didSave-follow-up admission state, not by folding
  `CpuBudgetSaturationSnapshot.did_save_followup_*` into generic saturation
  metrics;
- not require ad hoc generic saturation values such as
  `waiters_did_save_followup` or `permits_did_save_followup` for acceptance.

Representative live metrics or incident bundles captured for this change MUST
report both of these counters as absent or equal to `0` after this contract is
implemented:

- `intellisense_v2_observability_contract_violation_total`
- `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric`

#### Scenario: didSave follow-up saturation is visible without generic saturation violations

- **GIVEN** the didSave-follow-up admission state contains lane quota,
  active-slot, or queue-depth facts
- **WHEN** observability metrics are exported
- **THEN** generic runtime saturation gauges remain available for global
  interactive/background/shared budget and total queue-depth facts
- **AND** dedicated lane gauges expose `lane=did_save_followup` with bounded
  saturation metrics `quota`, `active_slots`, and `queue_depth`
- **AND** generic `queue_depth_total` does not silently absorb
  didSave-follow-up lane queue depth
- **AND** the export reports absent or zero
  `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric`

#### Scenario: new generic saturation values require complete projection

- **GIVEN** a runtime producer introduces a new generic `saturation_metric` value
- **WHEN** the value is not allowlisted or lacks drilldown/legacy projection
- **THEN** contract tests fail before the value can ship
- **AND** the value is not silently accepted as an unprojected metric

#### Scenario: representative bundle stays trustworthy for saturation analysis

- **GIVEN** a representative LSP observability bundle is captured after this
  change
- **WHEN** an operator reviews runtime saturation
- **THEN** `intellisense_v2_observability_contract_violation_total` is absent or
  zero
- **AND** `invalid_saturation_metric` is absent or zero
- **AND** didSave-follow-up lane saturation remains separately attributable
- **AND** no conclusion depends on reconstructing lane saturation from generic
  interactive/background buckets

#### Scenario: invalid lane-specific labels stay rejected on the generic path

- **GIVEN** a producer attempts to emit `waiters_did_save_followup` or
  `permits_did_save_followup` through the generic `SaturationGauge` path
- **WHEN** canonical event validation runs
- **THEN** the event is rejected as `invalid_saturation_metric`
- **AND** no generic drilldown or legacy saturation gauge is exported for that
  label
- **AND** the implementation must instead use the dedicated runtime-lane
  saturation family for didSave-follow-up lane visibility
