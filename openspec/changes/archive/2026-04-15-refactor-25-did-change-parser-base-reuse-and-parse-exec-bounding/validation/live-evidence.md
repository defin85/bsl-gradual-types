# Live Evidence

- `p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live` passed and wrote
  [refactor-25-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-25-real-conf-big-lagging-shadow-recovery-save-followup-live.json).
- The checked-in `conf_big` profile did not return to `ready_artifacts` under the mixed
  `didChange + didSave` load. The bounded residual remained truthful and specific:
  `followup_semantic_path=shadow_state`,
  `followup_ready_snapshot_zero_probe=not_ready`,
  `followup_ready_snapshot_wait_probe=timeout`,
  `followup_ready_snapshot_timeout_phase=parse_exec`,
  `followup_ready_snapshot_relief_valve_outcome=skipped_apply_lag`.
- The same report also shows `did_change_evidence_present=false` for version `4`, which matches the
  remaining live bottleneck: the exact worker still had not produced publishable same-version
  evidence before the `didSave` follow-up resolved through the truthful fallback path.
- Deterministic regressions cover the new guarantees that the live profile still stresses:
  bounded parser-base recovery success,
  truthful stale fallback when recovery fails,
  and `retargeted_during_parse` lifecycle attribution.
