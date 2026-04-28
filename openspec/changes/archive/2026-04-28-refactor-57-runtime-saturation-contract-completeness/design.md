## Context

The 2026-04-26 incident bundle on commit `8a406809` reports healthy completion
edges but also reports 180 observability contract violations:

```text
intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric=180
intellisense_v2_runtime_saturation_sample_total=720
```

Code inspection shows the mismatch:

- `emit_runtime_saturation_gauges` emits `waiters_did_save_followup` and
  `permits_did_save_followup` through the generic saturation-gauge event path.
- `ALLOWED_SATURATION_METRICS` accepts only global runtime-budget values:
  `waiters_interactive`, `waiters_background`, `permits_interactive`,
  `permits_background`, `permits_shared`, `queue_depth_total`.
- The dedicated runtime-lane family already exports
  `intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_*`.
- The current generic `queue_depth_total` is `interactive_waiters +
  background_waiters`; it does not include the dedicated didSave-follow-up lane
  queue.

External guidance supports keeping the taxonomy bounded and non-duplicative:

- Prometheus naming guidance says a metric should refer to one quantity and be
  understandable as an aggregate:
  https://prometheus.io/docs/practices/naming/
- OpenTelemetry metric semantic-convention guidance recommends consistent
  attributes and meaningful aggregation across attributes:
  https://opentelemetry.io/docs/specs/semconv/general/metrics/
- OpenMetrics recommends snake_case names, low ambiguity, and avoiding redundant
  metric-name information where labels/families already carry semantics:
  https://prometheus.io/docs/specs/om/open_metrics_spec/

## Architecture Drivers

- Telemetry truthfulness: contract violations in the metrics snapshot must mean
  a real producer bug, not a tolerated steady-state.
- Low cardinality: lane identity must stay bounded and explicit.
- Compatibility: existing global saturation gauges for interactive/background
  budget and total queue depth must remain stable.
- Scope control: this should not wait for the larger observability pipeline
  rewrite.
- Incident usefulness: exported bundles must let operators distinguish generic
  runtime budget saturation from dedicated didSave-follow-up lane saturation.

## Options

### Option A: Add didSave-follow-up values to the generic saturation allowlist

This would add `waiters_did_save_followup` and `permits_did_save_followup` to
`ALLOWED_SATURATION_METRICS` and projection tables.

Pros:
- Small code diff.
- Keeps the current producer calls mostly unchanged.

Cons:
- Mixes lane identity into the generic `saturation_metric` axis.
- Duplicates the existing dedicated runtime-lane family.
- Creates new legacy keys that were not actually exported from the failing
  shipped runtime because validation rejected them first.

### Option B: Keep didSave-follow-up saturation only in the dedicated lane family

This removes or reroutes the invalid generic emissions and treats
`did_save_followup` saturation as lane telemetry:

- `metric=quota`
- `metric=active_slots`
- `metric=queue_depth`
- `lane=did_save_followup`

Pros:
- Matches the existing dedicated lane contract.
- Avoids generic taxonomy growth for an orthogonal axis.
- Keeps global budget gauges and lane gauges separately attributable.
- Fixes the live violation without blocking on the full rewrite.

Cons:
- Requires tests to assert that the generic helper is not reused for
  lane-specific values.

### Option C: Wait for `rewrite-v2-observability-perf-pipeline`

This would defer the issue to the registry-compiled pipeline rewrite.

Pros:
- Long-term architecture is cleaner.

Cons:
- Current live bundles are already polluted with contract violations.
- Incident analysis cannot trust the violation counter while this remains open.

## Recommendation

Use Option B.

`did_save_followup` is a lane/admission identity, not a generic saturation
metric. The narrow fix should ensure:

1. Generic `SaturationGauge` emits only global runtime-budget gauges with full
   projection coverage.
2. Dedicated follow-up-lane saturation is emitted through the runtime-lane
   family.
3. Tests and live evidence fail if
   `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric`
   appears after the fix.

## Implementation Approach

1. In `emit_runtime_saturation_gauges`, stop publishing
   `waiters_did_save_followup` and `permits_did_save_followup` through
   `record_intellisense_v2_runtime_saturation_gauge_with_origin`.
2. Ensure the same snapshot still publishes dedicated lane saturation:
   `quota`, `active_slots`, and `queue_depth` for `lane=did_save_followup`.
   These gauges should remain owned by the didSave-follow-up admission state
   (currently `record_did_save_followup_lane_saturation_v2` or a semantically
   equivalent owner), not synthesized from `CpuBudgetSaturationSnapshot`.
3. Preserve the current generic `queue_depth_total` semantics. Do not silently
   add didSave-follow-up lane queue depth into that global compatibility gauge;
   lane queue depth remains visible through the dedicated lane family.
4. Add a targeted test that exercises both generic budget saturation and
   didSave-follow-up lane saturation, then asserts:
   - generic saturation gauges remain present for interactive/background/shared
     budget and total queue depth;
   - dedicated didSave-follow-up lane gauges are present;
   - no `invalid_saturation_metric` contract violation is emitted;
   - no accidental generic `*_did_save_followup` saturation keys are required for
     acceptance.
5. Add/adjust a low-level projection test so any future generic saturation value
   must be both allowlisted and projected before it can pass.
6. Add a negative test proving `waiters_did_save_followup` and
   `permits_did_save_followup` are still rejected if they are attempted through
   the generic saturation path, and that no generic drilldown/legacy keys are
   exported for those labels.
7. Capture a fresh representative metrics snapshot or incident bundle and attach
   evidence that `observability_contract_violation_total` and
   `invalid_saturation_metric` are absent or zero while `did_save_followup` lane
   gauges remain visible.

## Risks

- Risk: downstream consumers expected the never-exported generic
  `*_did_save_followup` keys.
  Mitigation: these keys were rejected by validation in the observed runtime;
  the stable exported surface is the dedicated lane family.

- Risk: removing generic attempts hides didSave-follow-up saturation.
  Mitigation: require explicit lane-family gauges in both tests and live
  evidence.

- Risk: `queue_depth_total` changes semantics by absorbing the dedicated lane
  queue.
  Mitigation: preserve the current compatibility gauge and require lane queue
  depth through the dedicated runtime-lane family.

- Risk: this overlaps with the larger observability rewrite.
  Mitigation: keep this as a current-pipeline repair and state that the rewrite
  must preserve the same bounded taxonomy.

- Risk: a future producer adds another saturation value without projection.
  Mitigation: add a fail-closed projection/allowlist completeness test for the
  current generic family.
