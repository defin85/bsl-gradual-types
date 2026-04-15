# Traceability

- Requirement `Background ready-snapshot workers are cooperatively superseded and exact-task promotable (MUST)`
  is covered by `cargo test -p bsl-backend --bin bsl-lsp-server p7_newer_did_change_cooperatively_supersedes_obsolete_ready_snapshot_worker -- --nocapture`
  and `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_promotes_already_queued_exact_worker_before_shadow_fallback -- --nocapture`.
- Requirement `didSave heavy follow-up avoids apply-lag as primary gate (MUST)` is covered by
  `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state -- --nocapture`
  , `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_promotes_already_queued_exact_worker_before_shadow_fallback -- --nocapture`,
  and `cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts -- --nocapture`.
- Requirement `bsl.getCurrentContext reuses exact same-version snapshot workers before independent parse (MUST)`
  is covered by `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_briefly_waits_for_equivalent_snapshot_worker_before_broker_parse -- --nocapture`
  , `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_superseded_generation_skips_obsolete_parse_and_stale_surface -- --nocapture`
  , `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_superseded_generation_keeps_completion_bounded_under_mixed_load -- --nocapture`
  and the broker-regression guard
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_get_current_context_same_revision_burst_shares_one_broker_leader_before_blocking -- --nocapture`.
- Mixed-load live evidence for `didSave` is captured in
  [refactor-20-real-conf-big-did-save-followup-runtime-live.json](/home/egor/code/bsl-gradual-types/openspec/changes/refactor-20-diagnostics-save-snapshot-worker-hardening/validation/refactor-20-real-conf-big-did-save-followup-runtime-live.json),
  produced by
  `cargo test -p bsl-backend --bin bsl-lsp-server p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live -- --nocapture`.
  The refreshed captured cycle still shows `followup_ready_snapshot_wait_probe=timeout`, but it
  advances into `followup_wait_reason=semantic_work` on `followup_semantic_path=shadow_state`
  instead of remaining blocked in a pure pending-publish / exact-wait starvation state, and it
  exposes no positive `followup_runtime_queue_wait_ms` residual at the same top-level trace.
- OpenSpec package validation passed via
  `openspec validate refactor-20-diagnostics-save-snapshot-worker-hardening --strict --no-interactive`.
