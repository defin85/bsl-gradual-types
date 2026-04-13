# Change: add live snapshot readiness visibility

## Why

`refactor-20` and the surrounding diagnostics/current-context work made snapshot-worker behavior more
truthful and debuggable for operators, but the product still lacks a user-facing answer to the
basic question: is the current document's exact snapshot ready right now, still rebuilding, or
already degraded to `shadow_state`?

Today that answer is hidden inside incident bundles, diagnostics-save timelines, and internal task
state. The VS Code extension only shows coarse startup/index readiness, and the read-only
`bsl-agent` HTTP UI only shows sessions/jobs rather than current snapshot readiness for tracked
documents.

## What Changes

- Add an authoritative live snapshot-readiness contract on the LSP side:
  - request `bsl.getSnapshotStatus`
  - notification `bsl/snapshotStatus`
- Add VS Code extension visibility for the active BSL document:
  - concise right-side status bar state
  - detailed snapshot-readiness section in existing observability UI
- Add a read-only `bsl-agent` parity HTTP surface and MCP UI section for snapshot readiness of
  session-tracked documents, using the same bounded vocabulary.
- Keep the change additive and visibility-focused: it exposes existing runtime truth rather than
  changing snapshot scheduling, wait budgets, or writer semantics.

## Sequence

This is a follow-up to the recent snapshot-worker/runtime hardening work, especially:

- `refactor-16-did-change-incremental-parse-fallback-attribution`
- `refactor-20-diagnostics-save-snapshot-worker-hardening`

Those changes made exact snapshot misses and fallbacks easier to diagnose. This change makes the
same runtime truth visible to normal users and operators without requiring incident export.

## Impact

- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
  - `mcp-bsl-agent`
- Affected code:
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `vscode-extension/src/lsp/*`
  - `vscode-extension/src/providers/*`
  - `bsl-api-dtos/src/dtos.rs`
  - `bsl-agent/src/http_ui/*`
  - unified read-only MCP/web UI in `frontend/`

## Non-Goals

- Do not change current snapshot-worker scheduling, supersession policy, or bounded wait budgets.
- Do not derive user-facing readiness from diagnostics-save timeline, completion timeline, or
  aggregate observability metrics.
- Do not add mutating UI controls like "force rebuild snapshot" or "cancel snapshot worker".
- Do not require tracking every file in the workspace; initial scope is the active LSP document and
  session-tracked documents in `bsl-agent`.
