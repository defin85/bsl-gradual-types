# Observability Evidence

## Goal

Close task `2.4` with representative evidence that runtime readiness observation no longer spends seconds in raw apply backlog before becoming a passive waiter, while actual apply lag remains independently attributable.

## Evidence

- Completion live gate:
  `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-revision-churn-completion-perf-live.json`
- didSave follow-up live gate:
  `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-did-save-diagnostics-followup-runtime-live.json`

## Completion mixed-load evidence

- Source commands:
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_completion_waiter_registration_bypasses_unrelated_interactive_apply_backlog -- --nocapture`
  `CHANGE_ID=refactor-12-runtime-ready-waiter-contention-bounding cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`
- Key signals from the live report:
  `summary.measured_latency_ms.p95 = 41`
  `summary.measured_head_hit_traces = 10`
  `summary.measured_prepare_timeout_wait_for_file_version_samples = 0`
  `summary.measured_interactive_wait_budget_exhausted_total_delta = 0`
  `summary.measured_wait_for_file_version_runtime_queue_wait_present_samples = 0`
  `summary.measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples = 10`

Interpretation:

- All measured completions resolved through `head_hit`.
- No measured completion regressed into `prepare_timeout@wait_for_file_version`.
- The runtime queue-wait field disappeared only because every measured sample was already fast-ready, not because the system stopped exposing the metric.
- Therefore the completion path now bypasses readiness-registration backlog cleanly when current-revision artifacts are already ready.

## didSave follow-up evidence

- Source commands:
  `cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_followup_stays_isolated_from_generic_background_reserved_blocker -- --nocapture`
  `CHANGE_ID=refactor-12-runtime-ready-waiter-contention-bounding cargo test -p bsl-backend --bin bsl-lsp-server p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live -- --nocapture`
- Key signals from the live report:
  `first_publish_elapsed_ms = 308`
  `first_publish_budget_ms = 2500`
  `followup_wait_reason = semantic_work`
  `idle_heavy_outcome = null`
  `followup_publish_kind = null`
  `observed_followup_runtime_queue_wait_ms = null`
  `followup_publish_runtime_queue_wait_ms = null`
  `followup_apply_lag_ms = 329`
  `followup_publish_apply_lag_ms = null`
  `terminal_outcome = null`

Interpretation:

- The bounded first publish stays comfortably inside its fast-lane budget.
- In this representative mixed-load run the richer follow-up remains in `semantic_work` instead of publishing a full payload, but it does so without reintroducing generic runtime queue-wait as the dominant blocker.
- Runtime queue-wait attribution drops out of the follow-up path, while apply lag remains visible as a separate residual signal.
- This still matches the intended separation for this change: readiness waiter registration is no longer the dominant tail, and any remaining delay is truthfully exposed as downstream semantic work and/or writer-owned apply lag rather than hidden inside raw FIFO residency.

## Conclusion

Together the completion and didSave artefacts show the intended post-change shape:

- readiness registration no longer produces the old seconds-scale `wait_for_file_version` queue-wait tail;
- completion stays on the current-revision fast path under representative revision churn;
- didSave follow-up keeps runtime queue-wait separate from actual apply lag and downstream heavy work, even when the representative run ends with explicit residual attribution instead of a richer follow-up publish.
