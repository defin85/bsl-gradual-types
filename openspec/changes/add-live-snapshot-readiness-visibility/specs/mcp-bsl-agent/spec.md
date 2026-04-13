## ADDED Requirements

### Requirement: `bsl-agent` parity API exposes read-only snapshot readiness for tracked documents

`bsl-agent` SHALL provide read-only HTTP endpoint `GET /api/mcp/snapshot-status` for snapshot
readiness of session-tracked documents.

The endpoint SHALL:

- follow the existing ready-session selection rules used by parity API;
- remain strictly read-only;
- define tracked documents as the deterministic union of session overlays and session hot-set
  entries;
- use the same bounded state vocabulary as LSP snapshot readiness wherever the semantics match;
- return enough information to distinguish building, exact ready, stale, `shadow_only`, and failed
  states for tracked documents.

Each entry SHALL include at least:

- document `path`
- session or analysis revision context
- `state`
- `exact`
- optional coarse `phase`
- `updatedAtMs`
- optional bounded `fallbackReason`

The response SHALL be deterministic for the same session state, including stable path ordering.

#### Scenario: Ready session reports building snapshot for a tracked document
- **GIVEN** a ready MCP session tracks a document whose exact snapshot is still rebuilding
- **WHEN** UI calls `GET /api/mcp/snapshot-status`
- **THEN** the endpoint returns an entry with `state=building`
- **AND** the response does not claim the tracked document is exact-ready

#### Scenario: No ready session keeps parity rule fail-closed
- **GIVEN** there is no ready session and the request omits `sessionId`
- **WHEN** UI calls `GET /api/mcp/snapshot-status`
- **THEN** the server returns `INVALID_PARAMS` / HTTP 400
- **AND** the response stays consistent with existing ready-session parity rules

#### Scenario: Ready session with no tracked documents returns an empty list
- **GIVEN** there is exactly one ready session
- **AND** that session currently has no overlays and no hot-set documents
- **WHEN** UI calls `GET /api/mcp/snapshot-status`
- **THEN** the server returns `200` with an empty entries list
- **AND** the response does not invent synthetic document rows

### Requirement: MCP UI shows snapshot readiness as read-only diagnostics

The unified SPA in MCP mode SHALL render snapshot readiness for tracked documents alongside
existing sessions/jobs diagnostics.

The MCP UI MUST:

- read snapshot readiness only from `/api/mcp/snapshot-status`;
- keep the surface strictly read-only;
- distinguish exact ready from `shadow_only` and `stale`;
- avoid mutating controls such as rebuild or cancel actions.

#### Scenario: MCP UI renders exact-ready and degraded states distinctly
- **GIVEN** MCP UI receives snapshot-status entries for tracked documents
- **WHEN** one document is exact-ready and another is `shadow_only`
- **THEN** the UI renders distinct labels for those states
- **AND** operators can tell that the degraded document is not exact-ready
