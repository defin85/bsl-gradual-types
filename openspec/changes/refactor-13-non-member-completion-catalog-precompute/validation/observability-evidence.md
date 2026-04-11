# Observability Evidence

## Goal

Close tasks `1.4` and `2.3` with representative live evidence that warm non-member completion now:

- exposes request-linked collect-stage source-family attribution directly in the completion timeline payload; and
- keeps the immutable deps-scoped families below the warm collect budget on the representative path.

## Evidence

- Live non-member completion report:
  `backend/tests/perf/reports/refactor-13-non-member-completion-catalog-precompute-real-conf-big-warm-non-member-collect-breakdown-live.json`

## Source Command

- `CHANGE_ID=refactor-13-non-member-completion-catalog-precompute cargo test -p bsl-backend --bin bsl-lsp-server p42_real_conf_big_warm_non_member_collect_breakdown_gate_live -- --nocapture`

## Key Signals

- `summary.measured_latency_ms = { count: 10, p95: 16 }`
- `summary.measured_ok_non_empty_traces = 10`
- `summary.measured_fail_closed_traces = 0`
- `summary.measured_contains_this_object_samples = 10`
- `summary.measured_trace_linked_samples = 10`
- `summary.measured_collect_breakdown_linked_samples = 10`
- `summary.measured_collect_ms = { count: 10, p95: 10 }`
- `summary.measured_collect_breakdown_ms.non_member_global_functions = { count: 10, p95: 0 }`
- `summary.measured_collect_breakdown_ms.non_member_metadata_items = { count: 10, p95: 7 }`
- `summary.measured_collect_breakdown_ms.non_member_repository_types = { count: 10, p95: 3 }`
- `summary.measured_collect_breakdown_ms.non_member_keywords = { count: 10, p95: 0 }`
- `summary.raw_collect_observability_ms = { count: 11, p95: 285 }`

## Interpretation

- The representative live path is a real `conf_big` form-module non-member identifier-tail probe (`marker = "Процедура ...\\n    ЭтотОбъек"`), so the report stays on the refactor-13 target path without the front-edge parse-gap confounder from `p42_real_conf_big_front_edge_completion_perf_report_live`.
- Every measured sample stays non-empty, preserves the contextual result `ЭтотОбъект`, links to a completion trace, and links to a `trace.collect_breakdown` payload. The gate therefore measures the intended warm non-member path rather than an `ok_empty` or histogram-only artifact.
- `summary.measured_collect_breakdown_ms.*` is now derived from request-linked timeline payload, while `summary.raw_collect_observability_ms` keeps the old process-wide histogram surface separate for operator debugging. This prevents raw cross-path histogram tails from polluting the measured warm-path acceptance budget.
- The collect budget is enforced directly on `summary.measured_collect_ms.p95`, so collect-stage regressions cannot hide behind aggregate completion totals.

## Conclusion

The live artifact now has the intended post-change shape:

- warm non-member completion stays successful on the representative path;
- collect-stage source-family attribution is exported per request through the completion timeline payload;
- measured acceptance uses trace-linked collect breakdowns, while raw process-wide histograms remain explicitly separate;
- immutable deps-scoped family attribution is enforced on every measured run with a direct warm collect budget gate.
