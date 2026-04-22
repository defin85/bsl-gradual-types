# Live Evidence

## Commands

- `cargo test -p bsl-analysis-v2 diagnostics_only_build_omits_exact_only_and_projection_only_fact_surfaces -- --nocapture`
- `cargo test -p bsl-analysis-v2 semantic_diagnostics_profiled_do_not_poison_later_exact_type_index_query -- --nocapture`
- `cargo test -p bsl-backend p7_diagnostics_only_query_keeps_exact_isolation_before_hover_and_definition_recovery -- --nocapture`
- `cargo test -p bsl-backend p7_hover_ -- --nocapture`
- `cargo test -p bsl-backend p7_definition_ -- --nocapture`
- `cargo test -p bsl-backend p7_signature_help_ -- --nocapture`
- `openspec validate refactor-42-restore-lsp-exact-consumers-after-diagnostics-only-simplification --strict --no-interactive`

## Result

- Analysis isolation still holds:
  - `diagnostics_only_build_omits_exact_only_and_projection_only_fact_surfaces` passed, so the
    diagnostics-only builder still omits exact-only targets and definition locations.
  - `semantic_diagnostics_profiled_do_not_poison_later_exact_type_index_query` passed, so a
    diagnostics-only query still does not publish or poison later exact type-index recovery.
- Runtime/LSP recovery now has direct narrowed-path coverage:
  - `p7_diagnostics_only_query_keeps_exact_isolation_before_hover_and_definition_recovery` passed.
    It forces a current revision into diagnostics-only state, proves an actual diagnostics query
    keeps the exact index unpublished, then proves same-revision `hover` and `definition` recover
    through the exact path rather than a diagnostics substitute.
  - `p7_hover_` passed, including bounded-success and fail-closed hover regressions.
  - `p7_definition_` passed, including bounded-success recovery and timeout fail-closed behavior
    for definition.
  - `p7_signature_help_` passed, covering the same exact-only family on the restored shared
    waiter path.
- Strict OpenSpec validation passed for the change.

## Interpretation

- The change now has end-to-end evidence that diagnostics-only artifacts remain non-substitutable
  while LSP exact consumers recover canonical current-revision exact semantics on the bounded
  default path.
- Fail-closed observability remains intact because the timeout regressions for hover, definition,
  and `signatureHelp` continue to pass after the shared exact-consumer recovery wiring.
