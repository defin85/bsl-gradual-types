# Live Evidence

## Commands

- `CHANGE_ID=refactor-21-did-change-ready-snapshot-coalescing cargo test -p bsl-backend --bin bsl-lsp-server p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p48_real_conf_big_coalesced_did_change_save_followup_report_live -- --nocapture`

## Evidence

- Mixed-load `conf_big` save follow-up still remains truthful under `didSave + documentSymbol` pressure:
  [refactor-21-real-conf-big-did-save-followup-runtime-live.json](/home/egor/code/bsl-gradual-types/openspec/changes/refactor-21-did-change-ready-snapshot-coalescing/validation/refactor-21-real-conf-big-did-save-followup-runtime-live.json)
  - `followup_ready_snapshot_task_state=in_flight_same_version`
  - `followup_ready_snapshot_wait_probe=timeout`
  - `followup_semantic_path=shadow_state`
  - `followup_wait_reason=semantic_work`
  - `followup_runtime_queue_wait_ms=null`

- Coalesced same-file `conf_big` burst now has a checked-in save-cycle reuse artifact:
  [refactor-21-real-conf-big-coalesced-did-change-save-followup-live.json](/home/egor/code/bsl-gradual-types/openspec/changes/refactor-21-did-change-ready-snapshot-coalescing/validation/refactor-21-real-conf-big-coalesced-did-change-save-followup-live.json)
  - `did_change_versions=[2,3]`
  - `did_change_worker_retargeted_before_parse_delta=1`
  - `did_change_worker_superseded_delta=0`
  - `did_change_worker_materialized_delta=1`
  - `followup_ready_snapshot_zero_probe=ready`
  - `followup_semantic_path=ready_artifacts`
  - `followup_semantic_parse_source=snapshot`
  - `followup_wait_for_file_version_ms=null`
  - `followup_publish_elapsed_ms=1619`

## Interpretation

- `p46` remains a truthful residual mixed-load scenario: the change does not claim that every `conf_big`
  save under concurrent pressure reaches exact `ready_artifacts`.
- `p48` shows the new evidence this change needed: once same-file churn is coalesced on a real
  `conf_big` module, the resulting exact save cycle can reuse `ready_artifacts` on the same save
  cycle without falling back to `wait_for_file_version`, while obsolete work is reported as
  `retargeted_before_parse` instead of generic `superseded`.
- `did_change_worker_started_delta=2` in `p48` is lifecycle-iteration based, not raw spawned-task
  count; the churn improvement is evidenced by `retargeted_before_parse=1`, `superseded=0`, and
  the successful `ready_artifacts` reuse outcome for the final exact revision.
