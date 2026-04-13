## ADDED Requirements

### Requirement: LSP publishes authoritative file-scoped snapshot readiness status (MUST)

LSP MUST provide an authoritative live snapshot-readiness contract for open documents through:

- custom request `bsl.getSnapshotStatus`
- custom notification `bsl/snapshotStatus`

The contract MUST stay file-scoped and MUST NOT be reconstructed from diagnostics save timeline,
completion timeline, or cumulative observability metrics.

The request/notification payload MUST use bounded low-cardinality fields including:

- `version`
- `uri`
- `requestedVersion`
- `readyVersion`
- `state` with bounded vocabulary `idle | building | ready | stale | shadow_only | failed`
- `exact`
- `taskState`
- optional coarse `phase`
- optional `trigger`
- `updatedAtMs`
- optional bounded `fallbackReason`

Notification updates MUST be coalesced per URI and MUST NOT emit unbounded micro-step noise for
every internal transition. Request fetch remains the hydrate/manual-read path.

#### Scenario: Same-version worker in flight reports building state
- **GIVEN** a matching same-version ready-snapshot worker is still in flight for an open document
- **WHEN** the client requests `bsl.getSnapshotStatus` for that document
- **THEN** the server returns `state=building`
- **AND** the payload stays truthful about the in-flight task state instead of claiming ready

#### Scenario: Exact ready snapshot reports exact-ready state
- **GIVEN** the requested document revision already has a matching ready snapshot
- **WHEN** the server serves snapshot readiness for that document
- **THEN** the payload reports `state=ready`
- **AND** `exact=true`
- **AND** `readyVersion` matches `requestedVersion`

#### Scenario: `shadow_only` fallback remains distinct from ready
- **GIVEN** the server can answer the current document only from `shadow_state` rather than exact
  ready snapshot artifacts
- **WHEN** snapshot readiness is reported
- **THEN** the payload reports `state=shadow_only`
- **AND** the payload does not claim exact readiness

#### Scenario: Live transition publishes coalesced notification
- **GIVEN** a document transitions from `building` to exact `ready`
- **WHEN** the server emits snapshot-readiness live updates
- **THEN** the client can observe the state change through `bsl/snapshotStatus`
- **AND** the server does not require timeline polling to surface that transition
