## 1. Shared Contract

- [ ] 1.1 Define a bounded snapshot-readiness DTO/vocabulary for LSP and MCP consumers, including
      state taxonomy, exactness, task state, coarse phase, and fallback semantics.
- [ ] 1.2 Keep the contract additive and fail-closed: unsupported transports must not infer
      readiness from retrospective observability surfaces.

## 2. LSP Runtime Surface

- [ ] 2.1 Add custom request `bsl.getSnapshotStatus` for file-scoped snapshot readiness.
- [ ] 2.2 Add custom notification `bsl/snapshotStatus` for coalesced live transitions on the same
      file.
- [ ] 2.3 Populate the contract from live ready-snapshot/task state so exact ready, in-flight,
      stale, `shadow_only`, and failure states remain truthful.
- [ ] 2.4 Add backend tests for request payloads and state transitions.

## 3. VS Code Extension

- [ ] 3.1 Add extension-side client plumbing for snapshot-status request/notification.
- [ ] 3.2 Surface active-document snapshot readiness in a right-side status bar item.
- [ ] 3.3 Add a snapshot-readiness detail section to the existing observability UI.
- [ ] 3.4 Add extension tests for supported, unsupported, exact-ready, building, and
      `shadow_only` states.

## 4. `bsl-agent` Read-only UI

- [ ] 4.1 Add read-only `/api/mcp/snapshot-status` under existing ready-session selection rules.
- [ ] 4.2 Render snapshot-readiness entries for session-tracked documents in MCP UI alongside
      existing sessions/jobs diagnostics.
- [ ] 4.3 Add tests for no-ready-session degradation and truthful MCP snapshot-state rendering.

## 5. Validation

- [ ] 5.1 Run `openspec validate add-live-snapshot-readiness-visibility --strict --no-interactive`.
- [ ] 5.2 Run the smallest relevant backend, extension, and agent verification covering the new
      status surfaces.
