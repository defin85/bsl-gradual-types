# Live Evidence

Command:

```bash
CHANGE_ID=refactor-31-diagnostics-save-exact-program-lowering-bounding cargo test -p bsl-backend p53_real_conf_big_exact_program_lowering_report_live -- --nocapture
```

Result:

- Test passed on `2026-04-15`.
- Raw repo-local capture was emitted to
  `backend/tests/perf/reports/refactor-31-diagnostics-save-exact-program-lowering-bounding-real-conf-big-exact-program-lowering-live.json`.
- A checked-in copy of the captured report is stored next to this note in
  `validation/refactor-31-real-conf-big-exact-program-lowering-live.json`.

Observed outcome on representative `examples/conf_big` mixed `didChange + didSave` load:

- `followup_semantic_path=shadow_state`
- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=program_lowering`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms=4047`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=4047`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms=null`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint=program_lowering`
- `program_conversion_coherent_with_program_lowering=true`

Interpretation:

- The representative mixed path still falls back to `shadow_state`; the remaining bounded residual
  is now truthfully localized to exact `program_lowering`.
- The exported aggregate stayed coherent with the dominant conversion slice in the same trace.
