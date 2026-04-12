# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| didChange parse-snapshot fallback MUST use bounded canonical reasons instead of generic `incremental_failed`. | `bsl-runtime/src/system/parser_coordinator.rs` `bsl-runtime/src/system/basic_observability.rs` `bsl-runtime/src/system/basic_observability/runtime_metrics.rs` | `cargo test -p bsl-runtime incremental_snapshot_reports_fallback_reason -- --nocapture` `cargo test -p bsl-runtime parse_snapshot_export_uses_only_canonical_fallback_buckets -- --nocapture` |
| didChange parse-snapshot builds MUST record producer-side base-text source and change-shape attribution for the requested version. | `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` `backend/src/bin/lsp_server/server/core.rs` `backend/src/bin/lsp_server/types.rs` | `cargo test -p bsl-backend p22_get_observability_metrics_exposes_did_change_parse_snapshot_evidence -- --nocapture` `cargo test -p bsl-backend p22_get_observability_metrics_exposes_input_edit_conversion_failure_reason -- --nocapture` |
| Incident bundle export MUST expose compact version-bound didChange parse-snapshot evidence that correlates with later didSave traces without raw text payloads. | `backend/src/bin/lsp_server/server/command_handlers.rs` `vscode-extension/src/providers/observabilityIncidentBundle.ts` `vscode-extension/src/providers/observabilityIncidentBundleParseSnapshot.ts` | `BSL_TEST_GREP='Observability Incident Bundle Test Suite' node ./vscode-extension/out/test/runTest.js` |

## Acceptance Closure

- As of 2026-04-12, the legacy exported counter key `intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_failed` is removed from the pre-registered observability contract.
- The active export surface now keeps only canonical didChange parse fallback buckets plus the version-bound evidence payload returned by `bsl.getObservabilityMetrics`.

## OpenSpec / Beads Sync

- `tasks.md` remains accurate for `1.1` through `3.1`; the reviewed contract drift is now closed in code and tests.
- Strict validation passes:
  `openspec validate refactor-16-did-change-incremental-parse-fallback-attribution --strict --no-interactive`
- Repo-local Beads state is present in this workspace and is accessible through `bd`.
- `bd show bsl-gradual-types-1rkq.2 --json` reports `status=closed` with `close_reason="Implemented and validated"` for the refactor-16 task.
- `bd show bsl-gradual-types-1rkq --json` reports the parent epic as still `open` because follow-up task `bsl-gradual-types-1rkq.3` remains open; this does not contradict the closed state of `bsl-gradual-types-1rkq.2`.
- The repo-local execution trace in `openspec/changes/refactor-15-diagnostics-save-ready-snapshot-miss-attribution/validation/epic-summary.md` remains consistent with the Beads state above.
