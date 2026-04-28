## p56 representative conf_big live gate

Command:

```bash
CHANGE_ID=refactor-52-didsave-parser-base-recovery-detached-ready-bounding BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-52-didsave-parser-base-recovery-detached-ready-bounding/validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture
```

Result: passed on rerun, 2026-04-24.

Test output:

```text
test server::core::tests::live_reports::p56_real_conf_big_diagnostics_representative_save_followup_bundle_live ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 462 filtered out; finished in 498.30s
```

Report:

```text
openspec/changes/refactor-52-didsave-parser-base-recovery-detached-ready-bounding/validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-live.json
```

The refactor-52 terminal-path contour passed in all captured cycles:

- `followup_semantic_path=detached_ready_artifacts` for all 4 captured cycles.
- `followup_semantic_path=shadow_state` count was 0.
- `followup_ready_snapshot_wait_probe=timeout` count was 0.
- `followup_did_save_exact_producer_final_lifecycle_state=detached_diagnostics_ready_published`
  for all 4 cycles.
- `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts` for all 4 captured cycles.

The rerun also passed the existing publish-latency ceiling:

- Baseline ceiling: `followup_publish_elapsed_ms <= 5219`.
- Observed max: `1666`.
- Ceiling delta: `-3553`.
- Max `followup_publish_semantic_diagnostics_query_ms`: `1496`.
- Max `followup_publish_publish_wait_ms`: `5`.
- Max `followup_ready_snapshot_parse_exec_ms`: `328`.
- Max `followup_ready_snapshot_bounded_wait_elapsed_ms`: `159`.

Initial cold-control repros:

- Empty disk-cache root:
  `BSL_CACHE_DIR=/tmp/bsl-refactor-52-p56-cold-cache.k0Tjpn`.
  Result: failed before JSON report write, full test time `580.82s`.
  Max `followup_publish_elapsed_ms`: `58772`; max `followup_publish_non_query_residual_ms`:
  `57573`; max `followup_publish_semantic_diagnostics_query_ms`: `1199`; max
  `followup_publish_publish_wait_ms`: `4`.
- Cache disabled:
  `BSL_CACHE_DISABLE=1`.
  Result: failed before JSON report write, full test time `578.44s`.
  Max `followup_publish_elapsed_ms`: `41166`; max `followup_publish_non_query_residual_ms`:
  `39976`; max `followup_publish_semantic_diagnostics_query_ms`: `1331`; max
  `followup_publish_publish_wait_ms`: `14`.

Both failed controls kept the refactor-52 terminal-path criteria green:
`detached_ready_artifacts`, no `shadow_state`, and final lifecycle
`detached_diagnostics_ready_published`. The failed controls proved this was not a stale parser-cache
artifact: clearing or disabling parser cache reproduced the same large residual.

Root-cause attribution:

- The first residual source was a telemetry/probe lock path: branch-context follow-up attribution
  could read the shared ready-state map while a same-version producer was still materializing,
  making `followup_before_branch_context -> followup_after_branch_context` absorb the producer wait.
- After that was removed from the follow-up telemetry path, the remaining cold/cache-disabled
  failure was a producer input race. Direct same-version `didSave` scheduling could create the exact
  producer before `didChange` post-handoff supplied ranged `parser_edits`. With no ranged edit and
  no cache, the producer fell into full `program_lowering` (`full_rebuild`, ~41-55s) even though the
  eventual terminal path was detached diagnostics-ready.

Fixed cold controls:

- Cache disabled:
  `BSL_CACHE_DISABLE=1`.
  Result: passed, full test time `524.95s`.
  Report:
  `validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-cache-disabled-live.json`.
  Max `followup_publish_elapsed_ms`: `1384`; max
  `followup_publish_non_query_residual_ms`: `68`; max
  `followup_publish_semantic_diagnostics_query_ms`: `1375`; max
  `followup_publish_publish_wait_ms`: `4`; max `followup_ready_snapshot_parse_exec_ms`: `184`.
  All four cycles reported `program_lowering_reuse_outcome=routine_body_reuse`.
- Empty disk-cache root:
  `BSL_CACHE_DIR=/tmp/bsl-refactor-52-p56-cold-cache-final.sq8Zp1`.
  Result: passed, full test time `559.48s`.
  Report:
  `validation/refactor-52-real-conf-big-diagnostics-representative-save-followup-bundle-cold-cache-live.json`.
  Max `followup_publish_elapsed_ms`: `1449`; max
  `followup_publish_non_query_residual_ms`: `87`; max
  `followup_publish_semantic_diagnostics_query_ms`: `1386`; max
  `followup_publish_publish_wait_ms`: `4`; max `followup_ready_snapshot_parse_exec_ms`: `193`.
  All four cycles reported `program_lowering_reuse_outcome=routine_body_reuse`.

Both fixed controls preserve the refactor-52 terminal-path criteria:
`followup_semantic_path=detached_ready_artifacts` for all cycles, no `shadow_state`, no bounded-wait
timeout, and final lifecycle `detached_diagnostics_ready_published`.
