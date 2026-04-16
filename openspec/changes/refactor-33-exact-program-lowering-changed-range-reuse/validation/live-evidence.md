# Live Evidence

## Commands

- `CHANGE_ID=refactor-33-exact-program-lowering-changed-range-reuse cargo test -p bsl-backend --bin bsl-lsp-server p53_real_conf_big_exact_program_lowering_report_live -- --nocapture`

## Result

- The representative `conf_big` live capture passed on `2026-04-16`.
- Raw repo-local capture was emitted to
  `backend/tests/perf/reports/refactor-33-exact-program-lowering-changed-range-reuse-real-conf-big-exact-program-lowering-live.json`.
- A checked-in copy of the capture is stored next to this note in
  `validation/refactor-33-exact-program-lowering-changed-range-reuse-real-conf-big-exact-program-lowering-live.json`.

Observed outcome on the traced `didChange + didSave` follow-up:

- `followup_publish_semantic_path=ready_artifacts`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms=32`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=32`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome=reused_prefix`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units=2088`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units=2`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count=1`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count=1`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units=2`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint=program_lowering`

## Comparison

- The proposal baseline cites the representative incident bundle captured on `2026-04-15T22:59:27Z`
  on `git c172fe76`, where `didSave` follow-ups still spent about `2569-2573ms` inside exact
  `program_lowering`.
- The refreshed local `p53` capture on `2026-04-16` shows `program_lowering_ms=32` while still
  publishing through `ready_artifacts`.
- The older checked-in local `refactor-32` `p53` sample recorded `program_lowering_ms=13`.
  This refreshed capture is therefore `+19ms` versus that older local sample, so that file should
  be treated as a different local capture rather than as the canonical incident baseline.

## Interpretation

- The refreshed evidence now distinguishes reused versus rebuilt exact lowering work on the traced
  target instead of exposing only wall-clock timing.
- The traced follow-up rebuilt only `2` lowering units while reusing `2088`, which matches the
  intended "bounded rebuild of the changed region" behavior.
- Against the incident-bundle baseline cited in the proposal, the refreshed exact
  `program_lowering` residual is materially lower while preserving `ready_artifacts` routing.
