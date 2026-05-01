# Change: add snapshot readiness diagnostics view

## Why
The VS Code status bar can currently report a compact state such as `BSL Snap: shadow-only v17`, but it does not explain why the exact snapshot is unavailable, whether a worker is still active, what failed last, or what the user can inspect next.

This makes snapshot failures and degraded `shadow_only` operation hard to diagnose during normal editing, even though the runtime already owns the authoritative file-scoped readiness signal.

## What Changes
- Extend the file-scoped `bsl/getSnapshotStatus` / `bsl/snapshotStatus` contract with bounded structured diagnostics for:
  - human-readable reason summary;
  - artifact readiness;
  - worker/task age and target;
  - cancellation/supersession;
  - last failure stage/reason;
  - status transition history support on the client.
- Upgrade the VS Code snapshot status UX:
  - keep the status bar compact;
  - make the status bar tooltip diagnostic rather than merely echoing state;
  - make the status bar click focus the snapshot readiness detail surface;
  - expand the existing Observability `Snapshot Readiness` tree into a real diagnostic view.
- Keep cache dashboard separate and add cross-links/actions only where useful; snapshot readiness stays file-scoped, while cache dashboard remains workspace/cache-scoped.
- Preserve fail-closed truthfulness: no UI surface may reconstruct snapshot readiness from diagnostics timelines, completion timelines, cache metrics, or cumulative observability metrics.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `bsl-api-dtos/src/dtos.rs`
  - `backend/src/bin/lsp_server/server/core/snapshot_status.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/lsp/snapshotStatus.ts`
  - `vscode-extension/src/providers/observabilityProvider.ts`
  - `vscode-extension/package.json`
  - VS Code extension tests and backend snapshot-status tests

## Non-Goals
- Do not change snapshot generation, exact type-index publication, or worker scheduling semantics.
- Do not replace the existing cache dashboard or merge cache health with file-scoped snapshot readiness.
- Do not add a Webview dashboard in the first implementation; the first surface should use VS Code status bar + Tree View APIs.
- Do not make `shadow_only` look equivalent to `ready`.
