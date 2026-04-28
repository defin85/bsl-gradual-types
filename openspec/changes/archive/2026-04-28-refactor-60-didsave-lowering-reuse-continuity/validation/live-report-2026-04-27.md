# Refactor-60 Live Validation Report

Captured: 2026-04-27

Command:

```bash
BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=backend/tests/perf/reports/refactor-60-didsave-lowering-reuse-continuity-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
```

Result: passed.

Raw report path:

```text
/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-60-didsave-lowering-reuse-continuity-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

The raw report is intentionally ignored by git through
`backend/tests/perf/reports/*`; this file records the durable validation
summary.

Key evidence:

- 4 representative `conf_big` save cycles completed.
- `max_first_publish_elapsed_ms=99`, below the 1000 ms first-publish ceiling.
- `max_first_publish_syntax_query_ms=73`, below the 1000 ms syntax-query ceiling.
- `max_followup_ready_snapshot_parse_exec_ms=165`, below the baseline ceiling.
- All 4 cycles used `program_lowering_reuse_outcome=routine_body_reuse`.
- All 4 cycles reported `program_lowering_reused_lowering_units=2079` and
  `program_lowering_rebuilt_lowering_units=9`.
- All 4 cycles reported `program_lowering_reuse_seed_source=ast_cache_owned`,
  `program_lowering_reuse_seed_candidate_count=1`, and
  `program_lowering_reuse_plan_build_source=owned`.
- No representative cycle had an unproved seconds-scale `program_lowering_tail`
  with `full_rebuild`, `0` reused units, all units rebuilt, and no seed source
  or failure reason.
- `observability_contract_violation_total=0`.
- `invalid_saturation_metric=0`.
- `runtime_saturation_sample_total=480`.

Scope note:

- The live report exercises the didSave representative large-module contour for
  this change. Completion transport ingress/egress code was not modified by
  refactor-60; completion transport health remains covered by the pre-change
  incident-bundle evidence linked in `validation/incident-bundle-2026-04-27T11-07-23Z.md`.
