# Change: refactor-07 diagnostics save followup apply-lag bounding

## Why

После `refactor-06-diagnostics-save-fastlane-queue-bypass-and-cycle-sequencing` live bundle от
`2026-04-07T20:23:03Z` показал, что `save_fastlane` first publish стал быстрым (`73-95ms`) и
перестал тратить секунды на shared interactive queue.

Но основной post-save tail никуда не делся:

- `intellisense_v2_wait_for_file_version_diagnostics_ms.p95=6924`;
- `intellisense_v2_runtime_wait_for_file_version_queue_wait_ms.p95=9837`;
- `intellisense_v2_runtime_apply_change_set_file_exec_ms.p95=7305`.

В diagnostics save timeline один cycle уже виден как `in_flight/pending`, а другой завершился
`idle_heavy_outcome=superseded_generation` без richer follow-up publish. Это означает, что
`didSave` heavy follow-up по-прежнему слишком сильно зависит от writer/apply lag.

## What Changes

- `didSave` follow-up heavy diagnostics больше не должен зависеть от unbounded apply-lag как от
  primary gate, если same-version save artifacts уже доступны.
- Система должна предпочитать same-version follow-up preparation path поверх ready artifacts вместо
  слепого ожидания `wait_for_file_version`.
- Diagnostics save timeline должен явно различать `followup pending because of apply-lag` от
  `followup pending because heavy semantic work still runs`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
