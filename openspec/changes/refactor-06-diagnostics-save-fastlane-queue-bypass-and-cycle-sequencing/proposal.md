# Change: refactor-06 diagnostics save fastlane queue bypass and cycle sequencing

## Why

Live bundle `2026-04-07T19:16:12Z` показал, что `didSave` first publish больше не скрыт, но всё ещё
может стоять секунды в `save_fastlane` fallback path из-за shared interactive blocking queue.

Тот же bundle показал operator-facing корреляционную аномалию: для одного `requested_version`
timeline может показывать save traces с неочевидным порядком `diagnostics_generation`, потому что
это общий generation diagnostics runtime, а не dedicated save-cycle identity.

## What Changes

- `save_fastlane` syntax-only shadow fallback больше не должен наследовать starvation от shared
  bounded interactive queue.
- diagnostics save timeline получает dedicated monotonic `save_cycle_sequence` для каждого
  `didSave` cycle.
- incident bundle и diagnostics save summary должны рендерить ordering/correlation через
  `save_cycle_sequence`, а `diagnostics_generation` оставлять как low-level supersession fact.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
