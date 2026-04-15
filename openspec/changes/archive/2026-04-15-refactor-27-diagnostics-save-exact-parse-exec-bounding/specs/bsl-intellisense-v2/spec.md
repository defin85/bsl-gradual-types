## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST keep exact `parse_exec` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside `parse_exec` whenever `didSave` heavy follow-up waits for an exact still-current
same-version ready-snapshot producer. The runtime MUST keep that exact path focused on
materializing current ready artifacts before optional in-parse work.

This behavior MUST:

- allow non-essential in-parse work to be deferred, skipped, or made cancellable until after exact
  ready artifacts are materialized;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before deferred enrichment

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `parse_exec`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes work required to materialize exact ready artifacts
- **AND** optional in-parse enrichment that is not required for the publishable ready snapshot does
  not block the first exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical producer

- **GIVEN** an exact same-version producer is already on the save-critical path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded in-parse checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `parse_exec` timeouts MUST expose bounded in-parse subphase attribution

The system MUST export a bounded in-parse subphase attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while inside `parse_exec`. This
attribution MUST identify which part of exact `parse_exec` dominated the miss. Operator-facing
observability MUST no longer stop at an opaque phase label when the remaining blocker is entirely
inside exact `parse_exec`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish save-critical parse/build work from deferrable or optional in-parse work;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific in-parse residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `parse_exec`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded in-parse subphase
- **AND** operator-facing output does not collapse the residual back into a single opaque
  `parse_exec` label

#### Scenario: Successful exact publish does not leave stale timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented in-parse attribution is absent or cleared
- **AND** the successful cycle does not report a stale parse timeout residual
