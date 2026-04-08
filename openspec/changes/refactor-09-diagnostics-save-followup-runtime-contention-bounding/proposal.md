# Change: Bound didSave follow-up runtime contention

## Why

Live bundle `bsl-observability-incident-2026-04-08T01-15-44Z` shows that recent save-refresh work
already fixed the earlier bottlenecks:

- `save_fastlane` first publish is bounded at `56ms`;
- `idle_heavy` follow-up already reuses syntax artifacts.

But the remaining `didSave` follow-up tail is still pathological on `conf_big`:

- final `idle_heavy` publish at `43224ms`;
- only `wait_for_file_version_ms=11580` and `semantic_diagnostics_query_ms=169` are attributed in
  the request-centric trace;
- cumulative metrics still show seconds-scale `runtime_queue_wait_interactive_ms` and
  `apply_change_set_file_exec_ms`.

This means the next bottleneck is no longer syntax cost. It is runtime/apply contention plus an
insufficiently detailed request-centric breakdown for the remaining tail.

## What Changes

- Rework `didSave + idle_heavy` follow-up so post-`save_fastlane` work does not inherit shared
  interactive/runtime contention as its default primary gate.
- Extend diagnostics save timeline with explicit follow-up runtime contention breakdown so terminal
  and in-flight save cycles do not collapse seconds-scale latency into an unexplained tail.
- Update incident bundle projection and live validation to show request-centric follow-up blocker
  facts instead of inferring them from cumulative histograms.

## Impact

- Affected specs: `bsl-intellisense`, `bsl-intellisense-v2`
- Affected code: diagnostics runtime, runtime/apply scheduling, diagnostics save timeline DTOs,
  observability incident bundle projection, diagnostics perf/regression validation
