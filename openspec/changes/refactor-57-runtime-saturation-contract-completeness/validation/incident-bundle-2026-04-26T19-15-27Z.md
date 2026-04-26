# Incident Bundle Evidence: 2026-04-26T19-15-27Z

## Source

- Bundle:
  `/home/egor/code/temp/bsl-observability-incident-2026-04-26T19-15-27Z`
- Runtime:
  `BSL Language Server 0.4.160 (build: 2026-04-26 21:59:47, git: 8a406809)`
- Binary:
  `/home/egor/code/bsl-gradual-types/vscode-extension/bin/lsp-server`
- Binary mtime:
  `2026-04-26T19:11:18.336Z`

## Observed Contract Violation

The metrics snapshot contains:

```text
intellisense_v2_observability_contract_violation_total=180
intellisense_v2_observability_contract_violation_reason_invalid_saturation_metric=180
intellisense_v2_runtime_saturation_sample_total=720
```

This is the concrete evidence for refactor-57. The violation is a telemetry
contract issue, not a completion latency regression.

## Saturation Surfaces In The Same Bundle

Global runtime saturation gauges exported successfully:

```text
intellisense_v2_runtime_saturation_waiters_interactive=0
intellisense_v2_runtime_saturation_waiters_background=0
intellisense_v2_runtime_saturation_permits_interactive=2
intellisense_v2_runtime_saturation_permits_background=1
intellisense_v2_runtime_saturation_permits_shared=4
intellisense_v2_runtime_saturation_queue_depth_total=0
```

Dedicated didSave-follow-up lane gauges were also visible:

```text
intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_quota=1
intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_active_slots=0
intellisense_v2_runtime_lane_saturation_gauge_origin_lsp_lane_did_save_followup_metric_queue_depth=0
```

The failing shape is therefore not missing lane visibility. It is the invalid
generic saturation attempt for lane-specific labels.

## Scope Decision

This evidence keeps refactor-57 narrow:

- repair the saturation contract violation;
- preserve generic runtime saturation compatibility gauges;
- preserve dedicated didSave-follow-up lane gauges;
- do not reopen `refactor-56-didchange-ready-materialization-baseline-bounding`;
- do not implement `rewrite-v2-observability-perf-pipeline`.
