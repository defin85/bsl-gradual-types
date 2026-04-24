## Historical p56 representative conf_big live gate failure

Superseded by the passing 2026-04-24 run recorded in:

```text
openspec/changes/refactor-51-didsave-exact-producer-lane-bounding/validation/refactor-51-real-conf-big-diagnostics-representative-save-followup-bundle-live.md
```

The failed evidence below is retained to preserve the residual that drove the final scheduler
preemption fix.

Command:

```bash
cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
```

Historical result: failed after 378.59s.

The functional detached-ready gate now passes in the captured cycles:

- 4/4 cycles used `followup_semantic_path=detached_ready_artifacts`.
- 4/4 cycles used `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`.
- 4/4 cycles exported `followup_did_save_exact_producer_lifecycle_state=detached_diagnostics_ready_published`.
- No captured cycle exported `followup_ready_snapshot_timeout_phase`, `followup_ready_snapshot_timeout_leaf`,
  `shadow_state`, or `generic_pipeline` as the terminal follow-up path.

Remaining residual: the representative perf ceiling still fails. The gate expects the
2026-04-18T18:52:50Z publish baseline ceiling of 5219ms when detached samples exist, but the latest
run observed `observed_max=38090`.

Key cycle evidence:

- cycle 1: `followup_publish_elapsed_ms=1116`, `followup_ready_snapshot_parse_exec_ms=128`,
  `followup_publish_semantic_diagnostics_query_ms=978`
- cycle 2: `followup_publish_elapsed_ms=1057`, `followup_ready_snapshot_parse_exec_ms=145`,
  `followup_publish_semantic_diagnostics_query_ms=901`
- cycle 3: `followup_publish_elapsed_ms=36531`, `followup_ready_snapshot_parse_exec_ms=163`,
  `followup_publish_semantic_diagnostics_query_ms=934`, `followup_publish_non_query_residual_ms=35597`
- cycle 4: `followup_publish_elapsed_ms=38090`, `followup_ready_snapshot_parse_exec_ms=143`,
  `followup_publish_semantic_diagnostics_query_ms=912`, `followup_publish_non_query_residual_ms=37178`

Interpretation: producer-owned detached readiness is now reached before fallback, but OpenSpec task
4.2 remains open because later cycles still contain a large non-query follow-up publish residual
outside the representative perf ceiling.
