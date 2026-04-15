# Verification

Completed commands:

```bash
cargo test -p bsl-syntax
cargo test -p bsl-runtime parse_snapshot_tests
cargo check -p bsl-backend --bin bsl-lsp-server
cargo test -p bsl-backend p30_
cargo test -p bsl-backend p31_did_change_revision_is_retargeted_during_program_lowering_inside_single_large_callable_body_when_newer_target_arrives
cargo test -p bsl-backend p29_diagnostics_save_timeline_reports_exact_ready_snapshot_assembly_checkpoint_for_exact_worker
cargo test -p bsl-backend p30_diagnostics_save_timeline_reports_publishable_artifact_packaging_checkpoint_for_exact_worker
cargo test -p bsl-backend p31_diagnostics_save_timeline_repeated_probe_snapshots_keep_exact_ready_snapshot_view_coherent
CHANGE_ID=refactor-31-diagnostics-save-exact-program-lowering-bounding cargo test -p bsl-backend p53_real_conf_big_exact_program_lowering_report_live -- --nocapture
openspec validate refactor-31-diagnostics-save-exact-program-lowering-bounding --strict --no-interactive
```

Notes:

- `4.2` was evaluated as not requiring a dedicated VS Code test run because this change did not
  alter diagnostics-save timeline contract versioning, request shape, or incident-bundle renderer
  logic; it only changed runtime control and how already-exported backend timing fields are merged.
- The authoritative backend timeline regressions for the operator-facing surface are
  `p29`, `p30`, and `p31`.
- The single-large-body regression is
  `p31_did_change_revision_is_retargeted_during_program_lowering_inside_single_large_callable_body_when_newer_target_arrives`.

## Post-review closure rerun

Commands rerun locally on `2026-04-15` while closing the reviewed runtime-regression gap:

```bash
cargo test -p bsl-runtime parse_snapshot_tests
cargo test -p bsl-backend p31_diagnostics_save_timeline_repeated_probe_snapshots_keep_exact_ready_snapshot_view_coherent
cargo test -p bsl-backend p31_did_change_revision_is_retargeted_during_program_lowering_inside_single_large_callable_body_when_newer_target_arrives
openspec validate refactor-31-diagnostics-save-exact-program-lowering-bounding --strict --no-interactive
```

Result:

- All rerun commands passed.
- The runtime regression
  `save_critical_requested_during_program_lowering_returns_before_packaging_checkpoint`
  now passes inside `cargo test -p bsl-runtime parse_snapshot_tests`.
