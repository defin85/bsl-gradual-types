# Change: restore runtime saturation contract completeness

## Why

The live bundle
`/home/egor/code/temp/bsl-observability-incident-2026-04-26T19-15-27Z`
was captured from the freshly published `8a406809` runtime and shows
`intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric=180`.

The failure is not a completion/UI latency regression. It is an observability
integrity gap: runtime policy emits `waiters_did_save_followup` and
`permits_did_save_followup` through the generic saturation-gauge canonical
surface, while the generic `ALLOWED_SATURATION_METRICS` registry intentionally
contains only global runtime-budget values. The dedicated `did_save_followup`
lane already has a separate runtime-lane metric family, so the fix should make
that lane surface complete and stop treating lane identity as a generic
saturation metric.

## What Changes

- Add `bsl-intellisense-v2` requirements that runtime saturation taxonomy stays
  fail-closed and complete:
  - generic saturation gauges accept only global runtime budget metrics with
    deterministic drilldown/legacy projection;
  - dedicated lane saturation for `did_save_followup` is exported through the
    bounded runtime-lane family, not through ad hoc generic metric names;
  - representative observability snapshots MUST have zero
    `invalid_saturation_metric` contract violations.
- Tighten tests around `emit_runtime_saturation_gauges` and
  `BasicObservability` projection so invalid saturation labels cannot ship
  silently.
- Preserve the broader `rewrite-v2-observability-perf-pipeline` as a future
  registry rewrite; this change is the narrow repair for the currently shipped
  telemetry contract violation.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/basic_observability/core_metrics.rs`
  - `bsl-runtime/src/system/basic_observability/runtime_metrics.rs`
  - `bsl-runtime/src/system/basic_observability/tests.rs`
  - representative bundle/metrics validation for
    `bsl.getObservabilityMetrics` or equivalent live LSP snapshot

## Non-Goals

- Do not reopen `refactor-56-didchange-ready-materialization-baseline-bounding`;
  the new bundle keeps the didChange materialization baseline healthy.
- Do not implement the full `rewrite-v2-observability-perf-pipeline` registry
  migration in this change.
- Do not add a third `CpuWorkClass`; `did_save_followup` remains an orthogonal
  lane/admission surface.
- Do not solve the remaining `didSave` follow-up `ready_install` latency here;
  this change repairs telemetry truthfulness first.
