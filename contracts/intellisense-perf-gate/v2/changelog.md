# intellisense-perf-gate v2

## 2.0.0

- Promotes the checked-in perf contract to the authoritative representative matrix that is actually shipped in the repository today:
  - profiles: `small`, `large`, `churn`
  - operations: `completion`, `hover`, `definition`, `type_at_position`, `members`
  - fixture families: `steady_member_chain`, `post_did_change_current_revision`,
    `object_module_explicit_context`, `recordset_module_explicit_context`,
    `incomplete_syntax_member_access`
  - latency metric families: `total_duration_ms`, `wait_for_file_version_ms`,
    `snapshot_preparation_ms`, `ir_query_ms`
  - resource metric families: `allocations_per_request`, `allocated_bytes_per_request`,
    `lock_wait_ms_per_request`, `lock_contention_events_per_request`
- Adds machine-readable operation matrix coverage and anti-rescue zero-budget guardrails.
- Adds per-operation zero-budget `fail_closed_total` / `fail_closed_rate` ceilings for the
  representative matrix so mandatory `hover` / `definition` / `type_at_position` paths cannot
  pass cutover acceptance while silently returning fail-closed responses.
- Adds checked-in `relative_ratio_baseline_floors` so authoritative blocking runs stay
  sensitive to real regressions while ignoring sub-floor latency jitter on canonical fast paths.
- Clarifies that authoritative relative-ratio blocking is keyed off `p95`; `p99` remains
  reported and protected by absolute ceilings, while `snapshot_preparation_ms` uses a `5ms`
  ratio floor to avoid false regressions from low-millisecond churn jitter.
- Declares the checked-in `v2` contract as `representative_matrix` and
  `authoritative_for_cutover_acceptance = true`.

Migration note: `v2` is now the canonical cutover perf contract. Baselines and reports MUST carry
the representative fixture/operation matrix and fail-closed anti-rescue evidence; completion-only
assets are no longer sufficient for authoritative cutover acceptance.
