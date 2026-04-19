# Change: restore LSP exact consumers after diagnostics-only semantic simplification

## Why

After the recent diagnostics-only simplification work, user-visible LSP exact features regressed:

- `textDocument/hover` no longer returns the expected semantic result on cases that worked before;
- `textDocument/definition` / F12 navigation also regressed.

The current repository contract makes this regression class concrete.

Current accepted behavior already states that:

- full `SemanticFacts` remain the exact semantic contract for interactive exact features such as
  hover, definition, `signatureHelp`, and type-at-position;
- diagnostics-only artifacts must stay isolated from the full exact semantic cache identity so a
  narrowed diagnostics artifact cannot poison later interactive exact requests.

Current code and tests show why this regression is plausible:

- diagnostics-only build intentionally does **not** materialize exact call targets, exact member
  targets, constructor targets, or definition locations;
- existing analysis-level isolation tests prove that diagnostics-only should not publish exact type
  index artifacts by accident;
- but those tests are narrower than a real LSP user flow where hover/F12 are invoked after the
  simplification on the same revision.

So the most likely incident class is not "diagnostics are wrong".
It is "diagnostics-only simplification or its runtime wiring broke the recovery path for LSP exact
consumers that still require canonical exact semantics."

That is an inference from the current code/spec/test surface, not yet a confirmed root cause.
The new change must therefore restore the contract first and close the acceptance gap with direct
LSP regressions.

## What Changes

- Require LSP exact consumers (`hover`, `definition`, and the same exact-only request family that
  shares their runtime path) to keep reaching canonical current-revision exact semantics after
  diagnostics-only simplification through the existing bounded exact-consumer policy.
- Require diagnostics-only simplification to remain non-substitutable for exact LSP consumers:
  restoring hover/F12 MUST happen by preserving or repairing the exact path, not by widening the
  diagnostics-only artifact until it silently becomes the new exact contract.
- Require the fix to preserve the existing LSP serve-only / fail-closed contract for exact type
  index readiness: the change MUST NOT reintroduce hidden on-demand exact materialization on the
  LSP request path as the rescue mechanism.
- Require direct backend/LSP regressions for same-revision hover and goto-definition after a
  diagnostics-only query or equivalent narrowed-path setup, because current analysis-only
  isolation tests are insufficient acceptance for this user-facing regression.
- Require refreshed validation evidence showing that hover/F12 work again without regressing
  fail-closed semantics when exact current-revision artifacts are genuinely unavailable.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/lib/analysis_api.rs`
  - `analysis-v2/src/lib/snapshots.rs`
  - `analysis-v2/src/lib/tests.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/type_system/services/hover_service.rs`
  - `bsl-runtime/src/application/type_system/services/definition_service.rs`
  - `bsl-runtime/src/application/type_system/services/signature_help_service.rs`
  - `backend/src/bin/lsp_server/server/core/execution_context.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`
  - backend/runtime regression tests and validation assets
- Follow-up relationship:
  - builds on `refactor-36-diagnostics-semantic-hints-split`
  - is intentionally separate from `refactor-40-diagnostics-only-semantic-query-bounding`,
    because this is a correctness regression on exact LSP features rather than a latency-only
    optimization
  - does not replace broad `add-lsp-functional-ga-readiness`; it restores a broken exact contract
    that GA readiness would otherwise merely observe
