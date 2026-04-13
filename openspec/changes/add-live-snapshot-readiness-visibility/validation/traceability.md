# Traceability: add-live-snapshot-readiness-visibility

## Summary

This change now has checked-in `Requirement -> Code -> Test` coverage for the mandatory runtime and
default UI paths:

- LSP snapshot-readiness request/notification contract
- VS Code active-document status bar + Observability snapshot details
- `bsl-agent` parity HTTP endpoint + read-only MCP UI snapshot-readiness rendering

## Requirement -> Code -> Test

| Requirement | Enforcing code path | Tests / evidence |
| --- | --- | --- |
| LSP publishes authoritative file-scoped snapshot readiness status | `backend/src/bin/lsp_server/types.rs`, `backend/src/bin/lsp_server/server/command_handlers.rs`, `backend/src/bin/lsp_server/server/core/snapshot_status.rs`, `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` | `backend/src/bin/lsp_server/server/core/tests.rs` (`snapshot_status_request_reports_exact_ready_for_matching_snapshot`, `snapshot_status_request_reports_building_for_matching_inflight_worker`, `snapshot_status_updated_at_is_monotonic_across_building_to_ready_transition`, `snapshot_status_request_reports_shadow_only_when_only_shadow_state_is_current`, `snapshot_status_request_reports_failed_when_last_build_aborted`) |
| VS Code extension shows active-document snapshot readiness truthfully | `vscode-extension/src/lsp/customRequests.ts`, `vscode-extension/src/lsp/snapshotStatus.ts`, `vscode-extension/src/lsp/client/lifecycle.ts`, `vscode-extension/src/providers/observabilityProvider.ts`, `vscode-extension/src/extension.ts` | `vscode-extension/src/test/suite/snapshotStatus.test.ts`, `vscode-extension/src/test/suite/observabilityProvider.test.ts` |
| `bsl-agent` parity API exposes read-only snapshot readiness for tracked documents | `bsl-agent/src/session/manager_session.rs`, `bsl-agent/src/http_ui/mod.rs` | `bsl-agent/src/session/tests.rs`, `bsl-agent/tests/http_ui_integration.rs` |
| MCP UI shows snapshot readiness as read-only diagnostics | `frontend/src/api/client.rs`, `frontend/src/app.rs` | `frontend/src/app.rs` (`mcp_snapshot_helpers_distinguish_exact_ready_from_shadow_only`), `bsl-agent/tests/http_ui_integration.rs` |

## Validation Commands

```bash
openspec validate add-live-snapshot-readiness-visibility --strict --no-interactive
cargo test -p bsl-backend snapshot_status_ -- --nocapture
cargo test -p bsl-agent http_snapshot_status_ -- --nocapture
cargo test -p bsl-agent --test http_ui_integration http_ui_ -- --nocapture
cargo test -p bsl-frontend mcp_snapshot_helpers_distinguish_exact_ready_from_shadow_only -- --nocapture
npm --prefix ./vscode-extension run compile:fast
(cd vscode-extension && BSL_TEST_GREP='Observability Provider Test Suite|Snapshot Status Test Suite' node ./out/test/runTest.js)
```
