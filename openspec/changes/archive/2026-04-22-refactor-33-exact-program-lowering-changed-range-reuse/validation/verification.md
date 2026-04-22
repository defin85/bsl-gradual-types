# Verification

Completed commands on `2026-04-16`:

```bash
cargo fmt --all
cargo test -p bsl-runtime exact_lowering_reuse_plan_ -- --nocapture
cargo test -p bsl-runtime exact_ready_snapshot_reuse_path_ -- --nocapture
cargo test -p bsl-runtime reused_program_lowering -- --nocapture
cargo test -p bsl-runtime exact_program_lowering_reuse_kill_switch_disables_reuse_plan -- --nocapture
cargo test -p bsl-backend p24b_diagnostics_save_timeline_exports_program_lowering_reuse_summary -- --nocapture
cargo test -p bsl-backend p32_ranged_did_change_program_lowering_retarget_preserves_parser_base_for_newer_target -- --nocapture
CHANGE_ID=refactor-33-exact-program-lowering-changed-range-reuse cargo test -p bsl-backend --bin bsl-lsp-server p53_real_conf_big_exact_program_lowering_report_live -- --nocapture
openspec validate refactor-33-exact-program-lowering-changed-range-reuse --strict --no-interactive
```

Notes:

- Runtime coverage confirms conservative reuse planning, fail-closed invalidation, full-parse
  equivalence for the bounded local-body path, truthful save-critical / cancel behavior during
  reused lowering, and the runtime-config kill switch.
- Backend coverage confirms that diagnostics-save timeline export now carries exact
  reuse-versus-rebuild summary fields and that ranged `didChange` retarget still preserves parser
  base continuity for the newest target.
- The representative live `p53` capture now exports the exact lowering reuse summary in addition to
  the existing `program_lowering` timing fields. See `validation/live-evidence.md`.
