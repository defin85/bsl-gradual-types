## ADDED Requirements

### Requirement: Snapshot readiness status exposes bounded diagnostics (MUST)
The LSP server SHALL extend the file-scoped snapshot readiness contract with optional structured diagnostic detail that explains degraded or failed states without requiring clients to reconstruct readiness from unrelated timelines or metrics.

The extended payload SHALL preserve the existing base fields and SHALL define bounded optional sections including:

- `reason`: a bounded reason code and short message for the current readiness state;
- `artifacts`: observable readiness of relevant file-scoped artifacts such as shadow state, ready parse snapshot, exact type index, and current completion/head artifact;
- `worker`: observable in-flight worker target, phase, trigger, age, cancellation, or supersession facts;
- `lastFailure`: the last explicit same-subject failure stage, reason, message, affected revision, and timestamp;
- `recommendation`: a bounded next-step hint for the UI.

For the schema version that introduces these diagnostics, the server SHALL populate each section when it owns authoritative data for that section. The server SHALL omit unknown diagnostic sections instead of inferring them. Missing diagnostic sections SHALL NOT imply readiness.

Diagnostic vocabularies used for artifact status, reason, failure stage, and recommendation SHALL be bounded and low-cardinality. Free-text failure detail MAY be included only as non-key detail text and SHALL be length-capped before client rendering or logging. Snapshot readiness payloads SHALL NOT include source text, full file contents, raw stack traces, or unbounded diagnostic blobs.

Clients SHALL remain compatible with the existing base payload. A response without diagnostic sections SHALL render those details as unknown or unavailable rather than as healthy.

#### Scenario: `shadow_only` explains the missing exact artifact
- **GIVEN** the active document has current shadow state for the requested revision
- **AND** no exact ready artifact exists for that revision
- **WHEN** the client requests `bsl/getSnapshotStatus`
- **THEN** the payload reports `state=shadow_only`
- **AND** the payload does not claim `exact=true`
- **AND** the diagnostic detail identifies that the answer is shadow-backed and names the exact artifact state as missing, stale, building, failed, or unknown when the server can observe it

#### Scenario: Failed snapshot reports stage and reason
- **GIVEN** the last attempted rebuild for a document revision ended in an explicit failure
- **WHEN** the client requests `bsl/getSnapshotStatus`
- **THEN** the payload reports `state=failed`
- **AND** the diagnostic detail includes the failed stage and bounded reason code
- **AND** any free-text failure message is presented as detail, not as a grouping key or state vocabulary extension

#### Scenario: Long-running build exposes worker age
- **GIVEN** a matching same-version snapshot worker is in flight
- **WHEN** the client requests `bsl/getSnapshotStatus`
- **THEN** the payload reports `state=building`
- **AND** the diagnostic detail includes the current worker phase when known
- **AND** the diagnostic detail includes worker age or start timestamp when known

#### Scenario: Legacy payload remains supported
- **GIVEN** the server returns the existing snapshot readiness base fields without diagnostic sections
- **WHEN** the VS Code client renders snapshot readiness
- **THEN** it still renders the base state, revision, exactness, and task fields
- **AND** it marks diagnostic sections as unknown or unavailable
- **AND** it does not infer missing artifact readiness from cache, diagnostics, completion timelines, or cumulative observability metrics

### Requirement: VS Code surfaces snapshot readiness diagnostics without replacing the source of truth (MUST)
The VS Code extension SHALL render snapshot readiness diagnostics from the authoritative snapshot status cache hydrated by `bsl/getSnapshotStatus` and `bsl/snapshotStatus`.

The status bar SHALL remain a compact signal for the active BSL editor. Its tooltip SHALL expose the most relevant diagnostic summary available for the current state, including reason, requested and ready revisions, exactness, worker detail, artifact summary, and last failure when present.

The status bar command SHALL focus or reveal the Snapshot Readiness detail surface. The detailed surface SHALL live in the existing Observability Tree View unless a later change explicitly justifies a Webview. If direct reveal is unavailable, the command SHALL focus the Observability view and refresh it rather than opening a new Webview.

The detailed Snapshot Readiness surface SHALL show:

- summary state and revisions;
- why the current state is degraded or failed;
- relevant artifact readiness;
- current worker/task detail;
- last explicit failure;
- recent accepted state transitions for the active URI, bounded by an explicit implementation limit;
- bounded actions such as refresh, prime exact index, open related timeline, or export an incident bundle.

#### Scenario: Status bar click opens details for `shadow_only`
- **GIVEN** the active BSL editor has snapshot status `state=shadow_only`
- **WHEN** the user activates the snapshot status bar item
- **THEN** VS Code reveals the Snapshot Readiness detail surface
- **AND** the detail surface shows why the active file is `shadow_only`
- **AND** the UI does not label the file as exact-ready

#### Scenario: Unsupported server remains explicit
- **GIVEN** the connected LSP server does not support `bsl/getSnapshotStatus`
- **WHEN** the snapshot readiness UI hydrates
- **THEN** the status bar and detail surface do not synthesize readiness from diagnostics, completion timelines, cache metrics, or cumulative observability metrics
- **AND** the UI reports snapshot readiness as unsupported or unavailable

#### Scenario: Recent transitions ignore stale updates
- **GIVEN** the client has accepted a snapshot status update for a URI with `updatedAtMs=T2`
- **WHEN** it receives an older update for the same URI with `updatedAtMs=T1`
- **AND** `T1 < T2`
- **THEN** the client ignores the older update
- **AND** the recent transition history does not regress to the older state

#### Scenario: Recent transitions stay bounded
- **GIVEN** the client receives more snapshot status updates for a URI than its configured transition-history limit
- **WHEN** the Snapshot Readiness detail surface renders recent transitions
- **THEN** it shows only the most recent accepted updates within that explicit limit
- **AND** reset or dispose clears retained transition history

### Requirement: Cache dashboard remains separate from file-scoped snapshot readiness (MUST)
The cache dashboard SHALL remain a workspace/cache-scoped surface and SHALL NOT own, synthesize, or override active-file snapshot readiness.

Cache dashboard actions MAY link to the Snapshot Readiness detail surface, but readiness truth SHALL continue to come from `bsl/getSnapshotStatus` / `bsl/snapshotStatus`.

#### Scenario: Cache metrics do not synthesize readiness
- **GIVEN** cache metrics are available for the workspace
- **AND** snapshot readiness is unsupported, unavailable, or reports `shadow_only`
- **WHEN** the user opens the cache dashboard
- **THEN** the cache dashboard does not present cache health as active-file exact readiness
- **AND** any link to snapshot diagnostics opens the dedicated Snapshot Readiness detail surface
