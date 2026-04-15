## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST keep exact `program_conversion` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside exact same-version `program_conversion` whenever `didSave` heavy follow-up is waiting on a
still-current producer. The runtime MUST keep that exact path focused on the minimum conversion
work required to materialize current ready artifacts before first exact publish.

This behavior MUST:

- allow secondary conversion or packaging work that is not required for the first exact ready
  snapshot to be deferred, skipped, or cancelled until after publish;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or newer save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before secondary conversion work

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `program_conversion`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes the conversion work required for publishable exact ready
  artifacts
- **AND** secondary conversion or packaging work that is not required for first publish does not
  block the first exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical conversion producer

- **GIVEN** an exact same-version producer is already on the save-critical conversion path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded conversion checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `program_conversion` timeouts MUST expose bounded conversion checkpoint attribution

The system MUST export a bounded conversion checkpoint attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while the dominant residual remains
inside `program_conversion`. Operator-facing observability MUST no longer stop at a monolithic
`program_conversion` bucket once that bucket becomes the dominant residual after `refactor-29`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish conversion / lowering slices from later packaging or ownership-handoff slices used by
  the final implementation;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific conversion residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `program_conversion`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded conversion checkpoint
- **AND** operator-facing output does not collapse the residual back into a single monolithic
  `program_conversion` bucket

#### Scenario: Successful exact publish does not leave stale conversion timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented conversion checkpoint attribution is absent or cleared
- **AND** the successful cycle does not report a stale exact conversion timeout residual
