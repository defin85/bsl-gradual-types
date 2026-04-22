## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST keep exact `program_lowering` bounded on the save-critical path

The system MUST treat exact same-version `program_lowering` as a bounded save-critical region
whenever `didSave` heavy follow-up is waiting on a still-current exact producer. The runtime MUST
not require a single monolithic lowering span to complete before save-critical promotion,
supersession checks, or the first publishable exact ready snapshot decision can take effect.

This behavior MUST:

- introduce bounded cooperative lowering checkpoints that the runtime can observe while exact
  lowering is still in progress;
- derive those checkpoints from actual lowering progress units (for example declaration, body, or
  bounded child batches) rather than only from wall-clock polling around one opaque lowering call;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve truthful supersession / retarget behavior when a newer same-file revision or newer save
  cycle overtakes the current target.

#### Scenario: Save-critical exact producer advances through bounded lowering checkpoints

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `program_lowering`
- **WHEN** runtime observes the next bounded lowering checkpoint
- **THEN** save-critical promotion and timeout attribution can react at that checkpoint
- **AND** the producer is not forced to remain invisible inside one monolithic lowering span

#### Scenario: Newer same-file target supersedes the bounded lowering producer

- **GIVEN** an exact same-version producer is already inside bounded `program_lowering`
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded lowering checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `program_lowering` attribution MUST remain internally coherent for one traced target

The system MUST export target-coherent, internally coherent conversion attribution whenever exact
same-version ready-snapshot work is in or times out inside `program_lowering`. Operator-facing
observability MUST not emit one diagnostics-save trace whose aggregate `program_conversion` timing
contradicts its own bounded conversion slices.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target and
  `save_cycle_sequence`;
- merge or replace conversion attribution as one target-coherent tuple rather than as independent
  per-field maxima gathered from multiple probe snapshots;
- guarantee that exported `program_conversion_ms` is absent or greater than or equal to every
  constituent conversion slice present in the same trace;
- prevent stale aggregate conversion timing from one traced target or probe snapshot from leaking
  into another target's final follow-up trace;
- keep dominant checkpoint identity and dominant duration derived from the same target-coherent
  attribution view as the exported aggregate and bounded slice fields;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Timeout inside `program_lowering` reports coherent aggregate and slice timings

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `program_lowering`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant lowering checkpoint truthfully
- **AND** `program_conversion_ms` is absent or greater than or equal to the reported
  `program_lowering_ms` and `publishable_artifact_packaging_ms`

#### Scenario: Repeated follow-up probe snapshots do not produce a self-contradictory final trace

- **GIVEN** the same `didSave` cycle records multiple follow-up probe snapshots while exact work is
  still moving through bounded conversion checkpoints
- **WHEN** diagnostics save timeline finalizes the operator-facing trace
- **THEN** the final trace keeps conversion aggregate, bounded slices, and dominant checkpoint
  coherent with one traced target
- **AND** the final trace does not merge stale aggregate timing with fresher per-slice maxima from
  another target state
