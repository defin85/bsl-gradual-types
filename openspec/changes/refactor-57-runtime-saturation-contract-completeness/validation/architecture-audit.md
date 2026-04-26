# Architecture Audit: refactor-57-runtime-saturation-contract-completeness

## Audit verdict

Approved to implement after OpenSpec review. The change is correctly scoped as a
telemetry-integrity repair, not a completion/UI latency fix and not a full
observability pipeline rewrite.

The audit found wording gaps in the initial proposal around counter acceptance,
`queue_depth_total` semantics, and the owner of dedicated didSave-follow-up lane
saturation. Those gaps were patched in the change artifacts before this audit
was recorded.

## Locked decisions

- Keep generic `SaturationGauge` for global runtime-budget metrics only:
  `waiters_interactive`, `waiters_background`, `permits_interactive`,
  `permits_background`, `permits_shared`, and `queue_depth_total`.
- Keep `did_save_followup` saturation in the dedicated runtime-lane family with
  stable lane identity and bounded metrics `quota`, `active_slots`, and
  `queue_depth`.
- Do not add `waiters_did_save_followup` or `permits_did_save_followup` to the
  generic saturation allowlist.
- Preserve the current `queue_depth_total` compatibility meaning as
  `interactive_waiters + background_waiters`; do not silently include dedicated
  didSave-follow-up lane queue depth.
- Source dedicated didSave-follow-up lane saturation from the admission owner
  (`record_did_save_followup_lane_saturation_v2` or equivalent), not from
  `CpuBudgetSaturationSnapshot`.
- Keep this change separate from `refactor-56` and from the larger
  `rewrite-v2-observability-perf-pipeline` migration.

## Evidence

- Incident bundle:
  `/home/egor/code/temp/bsl-observability-incident-2026-04-26T19-15-27Z`
- Runtime commit in that bundle: `8a406809`
- Observed violation:
  `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric=180`
- Current generic emitter:
  `bsl-runtime/src/application/intellisense_v2/policy.rs`
  `emit_runtime_saturation_gauges`
- Current generic saturation allowlist:
  `bsl-runtime/src/system/basic_observability.rs`
  `ALLOWED_SATURATION_METRICS`
- Current fail-closed validator:
  `bsl-runtime/src/system/basic_observability/core_metrics.rs`
  `invalid_saturation_metric`
- Existing dedicated lane owner:
  `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  `record_did_save_followup_lane_saturation_v2`

External guidance used:

- Prometheus metric naming guidance: one metric should represent the same
  logical thing across label dimensions, and split data when aggregation is not
  meaningful: https://prometheus.io/docs/practices/naming/
- OpenTelemetry metric semantic conventions: attributes and metric hierarchies
  should stay consistent and avoid semantic ambiguity:
  https://opentelemetry.io/docs/specs/semconv/general/metrics/
- OpenMetrics design considerations: avoid redundant metric-name information and
  keep metric-family semantics stable:
  https://prometheus.io/docs/specs/om/open_metrics_spec/

## Audit matrix

| Area | Verdict | Notes |
| --- | --- | --- |
| Requirement coverage | Pass after wording fixes | New spec covers generic saturation completeness, dedicated lane visibility, representative bundle trust, and negative generic-label behavior. |
| Runtime architecture fit | Pass | Option B matches the existing split between generic budget gauges and dedicated lane gauges. |
| Compatibility | Pass with guard | Existing generic key names stay stable; `queue_depth_total` semantics are now explicitly preserved. |
| Observability truthfulness | Pass with gate | Fresh evidence must show `observability_contract_violation_total` and invalid saturation reason absent or zero. |
| Performance | Low risk | Removing invalid generic emissions reduces rejected canonical events; no runtime scheduling behavior should change. |
| Operability | Pass | Operators keep global budget gauges and gain a clean violation counter for real future bugs. |
| Migration/rollback | Low risk | The rejected generic didSave-follow-up keys were not a trustworthy exported surface; dedicated lane family remains. |
| Test strategy | Pass after additions | Requires projection completeness, negative invalid-label test, policy/backend export coverage, and live snapshot/bundle evidence. |

## Execution plan

1. Update `emit_runtime_saturation_gauges` to stop emitting
   `waiters_did_save_followup` and `permits_did_save_followup` through the
   generic `SaturationGauge` path.
2. Preserve generic gauges and their exact current names.
3. Preserve dedicated didSave-follow-up lane gauges from the admission owner.
4. Add contract tests:
   - all accepted generic saturation metrics have drilldown and legacy
     projection;
   - generic didSave-follow-up labels are rejected and do not export keys;
   - policy/backend metrics export has no invalid saturation violation while
     lane gauges remain visible.
5. Capture fresh representative LSP metrics or incident-bundle evidence with
   absent-or-zero contract violation counters and visible didSave-follow-up lane
   saturation gauges.
6. Run focused tests, relevant backend/LSP metrics tests, `cargo check`, clippy
   if Rust production code changed, and strict OpenSpec validation.

## Exact wording fixes applied

- Replaced stale helper name `record_runtime_budget_saturation` with current
  `emit_runtime_saturation_gauges`.
- Normalized validation wording to `absent or 0` for zero-valued counters.
- Made `queue_depth_total` compatibility semantics explicit.
- Required dedicated lane facts to come from didSave-follow-up admission state,
  not from generic `CpuBudgetSaturationSnapshot`.
- Added a scenario that invalid lane-specific labels stay rejected on the
  generic path.

## Assumptions and open questions

- Assumption: downstream consumers did not depend on generic
  `*_did_save_followup` saturation gauges, because the observed runtime rejected
  them before export.
- Assumption: `record_did_save_followup_lane_saturation_v2` or its equivalent
  remains the owner of lane quota/active-slot/queue-depth telemetry.
- Open question for implementation: if any metrics consumer reports a real
  dependency on the rejected generic keys, document that consumer and decide
  whether it needs a compatibility migration in a separate change.
