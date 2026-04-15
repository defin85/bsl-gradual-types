# Live Evidence

## Commands

```bash
cargo test -p bsl-backend --bin bsl-lsp-server p23_ -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_timeline_records_wait_probe_ -- --nocapture
cargo test -p bsl-runtime did_save_followup_ready_snapshot_metrics_are_exported -- --nocapture
BSL_TEST_GREP='Observability Incident Bundle' npm test
CHANGE_ID=refactor-23-ready-snapshot-materialization-phase-attribution \
BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_PHASE_REPORT=/home/egor/code/bsl-gradual-types/openspec/changes/refactor-23-ready-snapshot-materialization-phase-attribution/validation/refactor-23-real-conf-big-ready-snapshot-phase-live.json \
cargo test -p bsl-backend --bin bsl-lsp-server p50_real_conf_big_ready_snapshot_phase_report_live -- --nocapture
```

## Findings

- `p23_diagnostics_save_timeline_reports_parse_exec_timeout_phase_for_exact_worker` and
  `p23_diagnostics_save_timeline_reports_post_parse_timeout_phase_for_exact_worker` prove that
  bounded-wait timeout attribution now distinguishes `parse_exec` from
  `post_parse_pre_materialization`.
- `p23_ready_snapshot_phase_attribution_separates_document_symbol_side_work` proves that
  `document_symbol_side_work` is exported as a separate non-readiness phase instead of inflating
  `ready_install`.
- Real `conf_big` evidence in
  `validation/refactor-23-real-conf-big-ready-snapshot-phase-live.json` shows the exact ready path
  stayed on `ready_artifacts`, reported `followup_ready_snapshot_dominant_phase=parse_exec`, and
  kept `followup_ready_snapshot_timeout_phase=null` on the successful path.
