## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST bound exact `parse_exec` residence before the first subphase callback

The system MUST bound the opaque pre-subphase `parse_exec` residence of a still-current
same-version exact ready-snapshot producer whenever `didSave` heavy follow-up is waiting on that
producer.

This behavior MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash, save_cycle_sequence)`
  target, or a semantically equivalent per-save-cycle identity;
- treat the region currently observable as `before_first_parse_exec_subphase` as part of the
  save-critical exact path rather than as an unbounded invisible entry span;
- either materially reduce that representative blocked interval or expose truthful bounded internal
  progress for the same target before the steady-state follow-up latency is dominated by that
  region;
- preserve the current bounded wait and relief-valve budgets as the primary latency envelope;
- NOT be satisfied solely by widening those budgets;
- NOT be satisfied solely by relabelling the same opaque interval under another observability
  bucket without reducing or truthfully subdividing it for the same target;
- preserve exact same-version semantics for any produced ready artifacts;
- preserve latest-wins supersession, retarget, and cancellation behavior when a newer same-file
  revision or newer save cycle overtakes the target;
- preserve operator-facing low-cardinality evidence distinguishing still-current continuation,
  exhausted continuation proof, supersession, and cancellation.

#### Scenario: Still-current same-version producer reaches bounded progress before opaque pre-subphase `parse_exec` dominates

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the representative timeout leaf would otherwise be `before_first_parse_exec_subphase`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** runtime executes the representative save-follow-up policy
- **THEN** the producer reaches a bounded first in-parse progress point or materializes exact ready
  artifacts in time for the representative path to avoid spending its steady-state latency inside
  one opaque pre-subphase `parse_exec` span
- **AND** the heavy follow-up remains on `ready_artifacts`

#### Scenario: Newer target still supersedes the pre-subphase producer truthfully

- **GIVEN** an exact same-version producer is still inside bounded pre-subphase `parse_exec`
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded checkpoint
- **THEN** the producer MAY terminate, retarget, or fall back truthfully instead of publishing
  stale output
- **AND** the system does not keep an obsolete target alive merely to avoid reporting pre-subphase
  attribution
