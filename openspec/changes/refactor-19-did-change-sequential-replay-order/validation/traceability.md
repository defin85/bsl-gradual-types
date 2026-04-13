# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| Ranged `didChange` producer MUST preserve LSP receive order and use one canonical replay plan for both `updated_text` and `parser_edits`. | `backend/src/bin/lsp_server/server/language_server/helpers.rs` `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server canonical_ranged_replay_plan_preserves_receive_order_for_incremental_parse -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p22_get_observability_metrics_exposes_incremental_mode_for_valid_multi_range_did_change -- --nocapture` |
| Valid sequential ranged `didChange` with `UTF-8 BOM + CRLF` MUST stay incremental and MUST NOT false-fallback to `edits_do_not_match_new_content`. | `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` `backend/src/bin/lsp_server/handlers/text_document.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p22_get_observability_metrics_exposes_incremental_mode_for_valid_bom_crlf_receive_order_did_change -- --nocapture` |
| didChange parse-snapshot evidence MUST export replay-order attribution and known base-version attribution for incident-bundle triage. | `backend/src/bin/lsp_server/types.rs` `backend/src/bin/lsp_server/server/core.rs` `backend/src/bin/lsp_server/server/mod.rs` `vscode-extension/src/lsp/customRequests.ts` `vscode-extension/src/providers/observabilityIncidentBundleParseSnapshot.ts` | `cargo test -p bsl-backend --bin bsl-lsp-server p22_get_observability_metrics_exposes_did_change_parse_snapshot_evidence -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p22_get_observability_metrics_exposes_input_edit_conversion_failure_reason -- --nocapture` `npm --prefix ./vscode-extension run compile:fast` `BSL_TEST_GREP='Observability Incident Bundle Test Suite' node ./vscode-extension/out/test/runTest.js` |
| Fresh live evidence MUST show that the target `conf_big` sequential ranged didChange path no longer false-fallbacks to `edits_do_not_match_new_content`. | `backend/src/bin/lsp_server/server/core/tests.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p47_real_conf_big_sequential_ranged_did_change_report_live -- --nocapture` `backend/tests/perf/reports/refactor-19-did-change-sequential-replay-order-real-conf-big-sequential-ranged-did-change-live.json` |

## Acceptance Closure

- As of 2026-04-12, `refactor-19` replaces the reverse-order assumption from `refactor-18` with spec-correct LSP receive-order replay for ranged `didChange`.
- The producer now uses one canonical receive-order replay plan for both local text reconstruction and the parser edit chain.
- Version-bound didChange parse-snapshot evidence now carries `contentChangesCount`, `replayOrder`, and `baseDocumentVersion` so incident bundles can distinguish replay drift from stale-base drift without raw payloads.
- Extension-side incident-bundle coverage now asserts that summary rendering preserves `base_document_version` when the server provides it, closing the last review gap on the default UX path.
- Fresh live `conf_big` evidence records `parse_mode=incremental`, `replay_order=receive_order`, `base_text_source=shadow_state`, `base_document_version=1`, and `fallback_reason=null`.

## OpenSpec / Beads Sync

- `tasks.md` now matches the implemented and validated state for `1.1` through `2.3`.
- `bd show bsl-gradual-types-zs97 --json` reports the change epic as `closed` with `close_reason="All child tasks completed and validated; closing refactor-19 epic."`
- `bd show bsl-gradual-types-zs97 --children --json` reports children `.1` through `.6` as `closed`, matching OpenSpec tasks `1.1` through `2.3`.
- Strict validation passes:
  `openspec validate refactor-19-did-change-sequential-replay-order --strict --no-interactive`
