# Change: restore truthful leaf attribution for the diagnostics-only semantic-facts path

## Why

Representative `p55` evidence after `refactor-36` confirms that the diagnostics-only path is
active and that the parser-side residual is no longer the dominant problem on this save-follow-up
path.

The latest live report on `2026-04-17` shows:

- `followup_publish_elapsed_ms=1371`
- `semantic_diagnostics_query_ms=1224`
- `semantic_diagnostics_ir_ms=837`
- `semantic_diagnostics_collect_ms=383`
- exact `program_lowering_ms=129`

That is already a clear win over the previous baseline, but it also shows that semantic
diagnostics remain the dominant residual.

The current diagnostics-only live report is still not truthful enough for the next optimization
step:

- it shows `ast_to_ir_convert_ms=214`;
- it shows `semantic_diagnostics_ir_ms=837`;
- but all former full-semantic-facts subphases are now `null`.

So roughly `~623 ms` of diagnostics-only IR work are now unattributed rather than explained.
Before starting another optimization change, the repo needs path-specific leaf attribution for the
diagnostics-only semantic-facts builder.

## What Changes

- Require a dedicated diagnostics-only semantic-facts build profile instead of exporting only the
  aggregate diagnostics-only IR total.
- Require representative save-follow-up observability to export path-specific leaf attribution for
  diagnostics-only semantic-facts work while keeping skipped full-semantic-facts leaves truthful.
- Require representative traced payloads to carry the diagnostics semantic materialization path
  directly, so diagnostics-only leaf fields are not interpreted only via cumulative metrics.
- Require refreshed representative `p55` evidence that compares the newly attributed
  diagnostics-only residual against the `refactor-36` baseline.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/lib/analysis_api.rs`
  - `analysis-v2/src/lib.rs`
  - `analysis-v2/src/lib/snapshots.rs`
  - `bsl-runtime/src/system/basic_observability/**`
  - `backend/src/bin/lsp_server/server/core/**`
  - `backend/src/bin/lsp_server/types.rs`
  - representative diagnostics-save reports/tests
- Follow-up relationship:
  - builds on `refactor-36-diagnostics-semantic-hints-split`
  - should land before any new optimization change for the diagnostics-only semantic residual
