# Validation Log

## Completed checks
- `openspec validate prioritize-completion-under-large-module-churn --strict --no-interactive` ✅
- `cargo test -p bsl-runtime --lib scale_aware` ✅
- `cargo test -p bsl-runtime --lib interactive_commands_preempt_background_backlog` ✅
- `cargo test -p bsl-runtime --lib background_commands_make_progress_under_interactive_flood` ✅
- `cargo test -p bsl-runtime --lib large_churn_transition_metric_is_low_cardinality` ✅
- `cargo test -p bsl-runtime --lib heavy_diagnostics_deferred_metric_normalizes_reason_and_profile` ✅
- `cargo test -p bsl-backend --bin bsl-lsp-server large_churn_` ✅
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_completion_after_did_change_does_not_hang` ✅

## Perf gate (completed)
`p31_scale_aware_large_small_completion_gate_live` was run in enforce mode:

```bash
BSL_V2_SCALE_AWARE_GATE_ENFORCE=1 \
BSL_V2_SCALE_AWARE_GATE_REPORT=openspec/changes/prioritize-completion-under-large-module-churn/validation/scale-aware-large-small-live.json \
cargo test -p bsl-backend --bin bsl-lsp-server p31_scale_aware_large_small_completion_gate_live -- --nocapture
```

- JSON report attached: `openspec/changes/prioritize-completion-under-large-module-churn/validation/scale-aware-large-small-live.json`.
- Result: `gate.pass=false`.
- Key ratios from report:
  - `large_completion_ratio=9.7148` (threshold `<=0.75`, fail)
  - `large_wait_ratio=0.0400` (threshold `<=0.6`, pass)
  - `small_completion_ratio=0.0140` (threshold `<=1.25`, pass)
- Dominant large warm stage: `ir_query_completion` (`p95=37768ms`), with `syntax_diagnostics_query` also high (`p95=37518ms`).
