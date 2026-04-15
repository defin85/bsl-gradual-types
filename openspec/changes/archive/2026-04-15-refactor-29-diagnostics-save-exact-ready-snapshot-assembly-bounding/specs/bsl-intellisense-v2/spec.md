## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST keep exact `ready_snapshot_assembly` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside exact same-version `ready_snapshot_assembly` whenever `didSave` heavy follow-up is waiting
on a still-current producer. The runtime MUST keep that exact path focused on the minimum assembly
work required to materialize current ready artifacts before first exact publish.

This behavior MUST:

- allow secondary assembly work that is not required for the first exact ready snapshot to be
  deferred, skipped, or cancelled until after publish;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or newer save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before secondary assembly work

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `ready_snapshot_assembly`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes the assembly work required for publishable exact ready
  artifacts
- **AND** secondary assembly work that is not required for first publish does not block the first
  exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical assembly producer

- **GIVEN** an exact same-version producer is already on the save-critical assembly path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded assembly checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `ready_snapshot_assembly` timeouts MUST expose bounded assembly checkpoint attribution

The system MUST export a bounded assembly checkpoint attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while the dominant residual remains
inside `ready_snapshot_assembly`. Operator-facing observability MUST no longer stop at a monolithic
`exact_ready_snapshot_assembly` bucket once that bucket becomes the dominant residual after
`refactor-28`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish conversion / packaging slices used by the final implementation;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific assembly residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `ready_snapshot_assembly`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded assembly checkpoint
- **AND** operator-facing output does not collapse the residual back into a single monolithic
  `exact_ready_snapshot_assembly` bucket

#### Scenario: Successful exact publish does not leave stale assembly timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented assembly checkpoint attribution is absent or cleared
- **AND** the successful cycle does not report a stale exact assembly timeout residual
