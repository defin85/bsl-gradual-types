# intellisense-perf-gate v2

## 2.0.0

- Keeps the machine-readable completion perf baseline that is actually shipped in the repository today:
  - profiles: `small`, `large`, `churn`
  - latency metrics: `completion_duration_ms`, `intellisense_v2_wait_for_file_version_completion_ms`,
    `intellisense_v2_snapshot_completion_ms`, `intellisense_v2_ir_query_completion_ms`
  - resource metrics: `allocations_per_completion`, `allocated_bytes_per_completion`,
    `lock_wait_ms_per_completion`, `lock_contention_events_per_completion`
- Adds explicit machine-readable coverage metadata instead of implying broader representative semantic coverage.
- Declares the current checked-in perf contract as `completion_only` and marks it as
  `authoritative_for_cutover_acceptance = false`.

Migration note: treat `v2` as the honest shipped baseline for the current checked-in perf harness,
not as proof of representative cross-operation cutover coverage. `hover`, `definition`,
`type_at_position`, and `members` are still absent from the checked-in perf contract/report path
and must not be inferred from `completion`-only evidence.
