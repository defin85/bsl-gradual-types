## Current source validation

Current checked-in source:

```text
git b9522b6f
```

The incident bundle was captured from installed runtime `git 00bcf03f`, so this validation checks
the newer workspace source rather than treating the bundle as current-runtime acceptance evidence.

## Code inspection

`save_fastlane` first publish now has three observable paths in
`backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`:

- applied-analysis syntax artifacts;
- ready-parse-snapshot syntax artifacts;
- fallback shadow-text `syntax_errors_only` recomputation.

The 2026-04-24T14:22:42Z slow first publish had `syntax_work_mode=recomputed` and
`syntax_diagnostics_query_ms=3397`, so the failure contour is the fallback syntax-only recompute
while the same-family exact producer is still in `parser_base_recovery`. The current representative
run no longer takes that path slowly: max first publish is `223ms` and max syntax query is `73ms`.

The same-version exact producer path now derives same-version parser edits and parser-base recovery
reuse from the latest ready snapshot before scheduling the didSave parse snapshot worker. The
checked current run proves the representative `program_lowering` path is reuse-based rather than a
full rebuild:

```text
program_lowering_reuse_outcome=routine_body_reuse
program_lowering_reused_lowering_units=2079
program_lowering_rebuilt_lowering_units=9
```

The `shadow_state` fallback still rejects the same-family `program_lowering full_rebuild` contour.
This change also adds p56 gate predicates so the corrected terminal path
`detached_ready_artifacts` is not enough when it arrives only after bounded-wait/relief timeout on
`program_lowering full_rebuild` with `0` reused units.

## Gate update

`backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs` now
fails all of these 2026-04-24 contours:

- `save_fastlane` first publish or `syntax_diagnostics_query_ms` over `1000ms`;
- terminal `detached_ready_artifacts` after bounded-wait or relief timeout while
  `timeout_phase=parse_exec`, `timeout_leaf=program_lowering`,
  `program_lowering_reuse_outcome=full_rebuild`, `reused_lowering_units=0`, and final lifecycle is
  detached-ready or fully materialized;
- the existing refactor-53 terminal `shadow_state` before later detached-ready/full
  materialization contour.

## Verification

```text
cargo test -p bsl-backend --bin bsl-lsp-server p56_refactor54_gate_predicates_reject_incident_contours -- --nocapture
```

Result:

```text
1 passed; 0 failed; finished in 0.00s
```

```text
CHANGE_ID=refactor-54-didsave-exact-materialization-latency-bounding \
cargo test -p bsl-backend --bin bsl-lsp-server \
  p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
```

Result:

```text
1 passed; 0 failed; finished in 437.32s
report: backend/tests/perf/reports/refactor-54-didsave-exact-materialization-latency-bounding-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

Representative report summary:

```text
followup_semantic_path_detached_ready_artifacts=4
followup_ready_snapshot_bounded_wait_winner_detached_ready_artifacts=4
followup_ready_snapshot_wait_probe_timeout=0
followup_ready_snapshot_program_lowering_full_rebuild_detached_ready_late_count=0
followup_ready_snapshot_program_lowering_full_rebuild_shadow_state_later_detached_count=0
save_fastlane_slow_first_publish_count=0
semantic_query_dominates_parse_exec_count=4
max_first_publish_elapsed_ms=223
max_first_publish_syntax_query_ms=73
max_followup_ready_snapshot_bounded_wait_elapsed_ms=47
max_followup_ready_snapshot_parse_exec_ms=163
max_followup_publish_elapsed_ms=2261
max_followup_publish_semantic_diagnostics_query_ms=2207
program_lowering_reuse_outcome=routine_body_reuse
program_lowering_reused_lowering_units=2079
program_lowering_rebuilt_lowering_units=9
```

OpenSpec validation:

```text
openspec validate refactor-54-didsave-exact-materialization-latency-bounding --strict --no-interactive
Change 'refactor-54-didsave-exact-materialization-latency-bounding' is valid
```

## Residual observation

The representative report still records high `did_change_ready_snapshot_materialization_ms`
histogram values (`p50=42597`, `p95=43758`) compared to the old p56 baseline. The refactor-54 gate
does not treat this as failure because the accepted contour is diagnostics-save first publish,
detached-ready bounded wait, follow-up publish, and `program_lowering` reuse. If canonical
didChange full-ready materialization latency becomes user-visible, it should be handled by a
separate scoped change rather than folded into this detached diagnostics-ready acceptance gate.
