# Live Evidence

## Commands

- `CHANGE_ID=refactor-40-diagnostics-only-semantic-query-bounding BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_LEAF_REPORT=openspec/changes/refactor-40-diagnostics-only-semantic-query-bounding/validation/refactor-40-real-conf-big-diagnostics-ready-snapshot-leaf-live.json cargo test -p bsl-backend --bin bsl-lsp-server p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live -- --nocapture`
- `CHANGE_ID=refactor-40-diagnostics-only-semantic-query-bounding BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-40-diagnostics-only-semantic-query-bounding/validation/refactor-40-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture`
- `cargo test -p bsl-analysis-v2 diagnostics_only_build_omits_exact_only_and_projection_only_fact_surfaces -- --nocapture`
- `cargo test -p bsl-analysis-v2 diagnostics_type_hints -- --nocapture`
- `cargo test -p bsl-analysis-v2 semantic_diagnostics_profiled -- --nocapture`
- `cargo test -p bsl-backend diagnostics_only_semantic_facts -- --nocapture`
- `openspec validate refactor-40-diagnostics-only-semantic-query-bounding --strict --no-interactive`

## Result

- `p55` leaf report:
  [refactor-40-real-conf-big-diagnostics-ready-snapshot-leaf-live.json](./refactor-40-real-conf-big-diagnostics-ready-snapshot-leaf-live.json)
  - semantic path stayed `ready_artifacts`
  - semantic materialization path stayed `diagnostics_only`
  - against checked-in `refactor-39` leaf baseline:
    - `followup_publish_elapsed_ms`: `1319 -> 1194`
    - `semantic_diagnostics_query_ms`: `1167 -> 1049`
    - `semantic_diagnostics_ir_ms`: `793 -> 683`
    - `semantic_diagnostics_collect_ms`: `370 -> 362`
  - against the previous reduced-output workspace cut:
    - `diagnostics_only_semantic_facts_ms`: `460 -> 442`
    - `local_function_summaries_ms`: `216 -> 194`
    - `visit_statements_ms`: `243 -> 247`
    - `visit_callable_body_ms`: `163 -> 166`
  - truthful observability stayed intact while dropping projection-only payload retention:
    - `index_entry_count=14034`

- `p56` representative family bundle:
  [refactor-40-real-conf-big-diagnostics-representative-save-followup-bundle-live.json](./refactor-40-real-conf-big-diagnostics-representative-save-followup-bundle-live.json)
  - family summary stayed identical to `refactor-39` on path identity:
    - `ready_artifacts=4`
    - `shadow_state=0`
    - `followup_ready_snapshot_wait_probe_ready=4`
    - `followup_ready_snapshot_zero_probe_not_ready=4`
  - representative latency moved down versus checked-in `refactor-39` bundle:
    - average `followup_publish_elapsed_ms`: `1455.75 -> 1242`
    - average `followup_publish_semantic_diagnostics_query_ms`: `1288 -> 1072.5`
    - average `followup_ready_snapshot_parse_exec_ms`: `151.75 -> 154.5`

## Interpretation

- The second builder cut replaces diagnostics-only projection-only `type_entries` retention with
  direct final-pass hint recording, while keeping the four-map hint surface and downstream
  diagnostics parity.
- The refreshed `p55` leaf report and `p56` representative bundle both move down against the
  checked-in `refactor-39` baseline without changing semantic path identity, so `2.3` is now
  satisfied rather than left open on a local-only win.
- Collector revisit is still not justified by the refreshed evidence because `collect_ms=362`
  remains below the current diagnostics-only builder residual `diagnostics_only_semantic_facts_ms=442`.
- An exploratory diagnostics-only type-entry span-filter branch was tested during this session and
  rejected: it improved one `p55` run locally but regressed representative `p56`, so it is not
  part of the current workspace state.
