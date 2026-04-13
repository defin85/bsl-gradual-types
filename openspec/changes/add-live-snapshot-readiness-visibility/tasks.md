## 1. Shared Contract

- [x] 1.1 Define a bounded snapshot-readiness DTO/vocabulary for LSP and MCP consumers, including
      state taxonomy, exactness, task state, coarse phase, fallback semantics, and a schema field
      that is distinct from document/session revision fields.
- [x] 1.2 Keep the contract additive and fail-closed: unsupported transports must not infer
      readiness from retrospective observability surfaces.

## 2. LSP Runtime Surface

- [x] 2.1 Add custom request `bsl.getSnapshotStatus` for file-scoped snapshot readiness.
- [x] 2.2 Add custom notification `bsl/snapshotStatus` for coalesced live transitions on the same
      file.
- [x] 2.3 Populate the contract from live ready-snapshot/task state so exact ready, in-flight,
      stale, `shadow_only`, and failure states remain truthful.
- [x] 2.4 Add backend tests for request payloads, monotonic per-URI `updatedAtMs`, and state
      transitions.

## 3. VS Code Extension

- [x] 3.1 Add extension-side client plumbing for snapshot-status request/notification.
- [x] 3.2 Surface active-document snapshot readiness in a dedicated right-side status bar item
      without reusing the existing left-side global BSL status/progress item.
- [x] 3.3 Add a snapshot-readiness detail section to the existing observability UI.
- [x] 3.4 Add extension tests for supported, unsupported, exact-ready, building, and
      `shadow_only` states, plus stale-notification dropping for the same URI.

## 4. `bsl-agent` Read-only UI

- [x] 4.1 Add read-only `/api/mcp/snapshot-status` under existing ready-session selection rules.
- [x] 4.2 Render snapshot-readiness entries for session-tracked documents in MCP UI alongside
      existing sessions/jobs diagnostics.
- [x] 4.3 Add tests for no-ready-session degradation, deterministic tracked-document ordering, and
      truthful MCP snapshot-state rendering.

## 5. Validation

- [x] 5.1 Run `openspec validate add-live-snapshot-readiness-visibility --strict --no-interactive`.
- [x] 5.2 Run the smallest relevant backend, extension, and agent verification covering the new
      status surfaces.
