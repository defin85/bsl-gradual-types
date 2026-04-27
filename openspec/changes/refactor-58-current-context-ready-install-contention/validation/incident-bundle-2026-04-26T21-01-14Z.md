# Incident Bundle Evidence: 2026-04-26T21-01-14Z

Source bundle:
`/home/egor/code/temp/bsl-observability-incident-2026-04-26T21-01-14Z`

Runtime source:

- `lsp_server` git: `0f3a07de`
- captured at: `2026-04-26T21:01:14.990Z`
- single URI bundle request count: `6`

## Integrity

- `intellisense_v2_observability_contract_violation_total=0`
- `intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric`
  is absent
- `intellisense_v2_runtime_saturation_sample_total=612`
- runtime-lane saturation for `did_save_followup` remains visible

This means the incident is not a continuation of
`refactor-57-runtime-saturation-contract-completeness`; the saturation contract
is clean in this bundle.

## Completion Path

The completion evidence does not justify treating VS Code UI/pre-send,
transport ingress, or output handoff as the primary suspect:

- client probes are small: `4ms`, `5ms`, `3ms`, `14ms`, `177ms`, `10ms`;
- `client_before_transport_write_wait_ms` is around `1-2ms`;
- one completion trace is `173ms` total and dominated by collection
  (`collect=172ms`);
- another completion trace is `2ms` total with
  `same_file_ingress_token_wait_ms=1ms`.

Concurrent `bsl.getCurrentContext` requests appear as first-poll contenders with
ages up to `3485ms`, but completions themselves are not blocked for seconds.

## didSave Follow-Up

Two same-file `didSave` follow-up traces are the important residual evidence:

- trace 1:
  - requested version: `11`
  - first publish: `65ms`
  - follow-up total: `1225ms`
  - `snapshot_with_deps=382ms`
  - semantic query: `840ms`
  - semantic path: `detached_ready_artifacts`
  - parse execution: `26ms`
- trace 2:
  - requested version: `15`
  - first publish: `53ms`
  - follow-up total: `2440ms`
  - `ready_install=2193ms`
  - `snapshot_with_deps=1949ms`
  - semantic query: `490ms`
  - semantic path: `ready_artifacts`
  - parse execution: `84ms`
  - dominant label: `ready_install`

The second trace is the core reason for this change: parse execution is small,
but the readiness/install portion is seconds-scale and too coarse to identify
the concrete blocker.

## Supporting Metrics

Cumulative metrics point at readiness/runtime waits rather than completion
transport:

- `intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms`:
  - count: `10`
  - `p95=3881ms`
- `intellisense_v2_runtime_wait_for_file_version_exec_ms`:
  - count: `12`
  - `p50=2071ms`
  - `p95=2680ms`
  - `p99=2705ms`
- `intellisense_v2_runtime_lane_exec_ms_origin_lsp_lane_did_save_followup`:
  - count: `8`
  - `p50=2440ms`
  - `p95=5508ms`
- `intellisense_v2_runtime_lane_queue_wait_ms_origin_lsp_lane_did_save_followup`:
  - count: `6`
  - `p95=421ms`
- ready-parse-snapshot materialization:
  - `did_change=2304ms`
  - `did_save=2434ms`
  - `did_open=8774ms`

## Scope Decision

This bundle should drive a backend readiness/current-context contention change:

- first-class per-request current-context evidence is missing;
- `ready_install` and `snapshot_with_deps` residuals need lower-level
  attribution or bounded behavior;
- completion and UI/pre-send are not the primary target for this incident;
- `shadow_state` terminal fallback from `refactor-50` is not the observed
  residual in this bundle.
