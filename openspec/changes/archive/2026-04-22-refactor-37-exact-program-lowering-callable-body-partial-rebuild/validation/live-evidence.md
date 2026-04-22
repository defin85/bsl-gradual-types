# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-runtime exact_lowering_reuse_plan_ -- --nocapture`
- `cargo test -p bsl-runtime exact_ready_snapshot_reuse_path_ -- --nocapture`
- `cargo test -p bsl-runtime save_critical_requested_during_reused_program_lowering_returns_before_packaging_checkpoint -- --nocapture`
- `cargo test -p bsl-runtime exact_ready_control_callback_can_cancel_during_reused_program_lowering -- --nocapture`
- `cargo test -p bsl-backend p24b_diagnostics_save_timeline_exports_program_lowering_reuse_summary -- --nocapture`
- `CHANGE_ID=refactor-37-exact-program-lowering-callable-body-partial-rebuild cargo test -p bsl-backend p53_real_conf_big_exact_program_lowering_report_live -- --nocapture`
- `CHANGE_ID=refactor-37-exact-program-lowering-callable-body-partial-rebuild cargo test -p bsl-backend p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live -- --nocapture`
- `openspec validate refactor-37-exact-program-lowering-callable-body-partial-rebuild --strict --no-interactive`

## Result

- Targeted parser/runtime regressions passed on `2026-04-17`.
- `p24b` passed and kept the diagnostics-save timeline export contract green.
- `p53` passed and emitted a raw repo-local capture to
  `backend/tests/perf/reports/refactor-37-exact-program-lowering-callable-body-partial-rebuild-real-conf-big-exact-program-lowering-live.json`.
- `p55` still failed its overall live gate, but the failure moved off the parser hot path and into
  semantic diagnostics work. The parser-side trace for the same representative target is still
  decisive evidence for this change.

## Representative Evidence

### `p53`

Observed on the refreshed representative exact-program-lowering path:

- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=182`

This run exited on the exact `program_lowering` observation path before the final idle-heavy
publish, so it did not export the richer reuse summary fields in the checked report. The command
still passed and refreshed the parser-side representative capture for this change.

### `p55`

Baseline parser-side trace from the pre-change `2026-04-17` investigation:

- `program_lowering_ms=1209`
- `reuse_outcome=top_level_reuse`
- `rebuilt_lowering_units=46`
- `routine_body_reuse_node_count=0`
- `rebuild_dispatch_callable_body_dispatch_ms=1201`
- `rebuild_dispatch_callable_body_dispatch_call_count=45`

Refreshed parser-side trace after the `refactor-37` callable-body partial-rebuild work:

- `program_lowering_ms=244`
- `reuse_outcome=routine_body_reuse`
- `rebuilt_lowering_units=9`
- `routine_body_reuse_node_count=1`
- `routine_body_reused_prefix_lowering_units=31`
- `routine_body_reused_suffix_lowering_units=6`
- `rebuild_dispatch_callable_body_dispatch_ms=217`
- `rebuild_dispatch_callable_body_dispatch_call_count=11`
- `rebuild_dispatch_control_flow_ms=217`
- `rebuild_dispatch_control_flow_call_count=1`
- `rebuild_dispatch_simple_ms=0`
- `rebuild_dispatch_simple_call_count=0`

The same `p55` command still fails its full publish budget because the remaining dominant work is
now semantic rather than parser-side:

- `followup_publish_elapsed_ms=3193`
- `semantic_diagnostics_query_ms=2451`
- `semantic_diagnostics_ir_ms=1611`

## Interpretation

- `refactor-37` achieved its parser-side goal on the representative `p55` path: the traced target
  no longer rebuilds the whole callable body, and the exact `program_lowering` residual dropped
  materially versus the pre-change trace.
- The refreshed `p55` evidence shows the intended shape change, from whole-callable rebuild
  (`top_level_reuse` with `45` callable-body dispatch units) to bounded callable-body rebuild
  (`routine_body_reuse` with `11` callable-body dispatch units).
- The remaining red `p55` budget is outside the parser scope of this change and now sits in
  semantic diagnostics. That residual should be handled by the semantic follow-up work rather than
  by widening parser-side rebuild boundaries further.
