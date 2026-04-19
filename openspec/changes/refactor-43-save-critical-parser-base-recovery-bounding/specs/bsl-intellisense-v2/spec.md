## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST keep exact `parser_base_recovery` on the save-critical path

The system MUST treat matching parser-base proof or recovery as save-critical work only to the
extent required to resume exact ready-snapshot materialization for the still-current same-version
target whenever `didSave` heavy follow-up is waiting on that target and the dominant exact blocker
remains `parser_base_recovery`.

This behavior MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- keep `parser_base_recovery` focused on bounded work required to prove or install a matching
  parser base for that exact target before later tree-build or exact-artifact work proceeds;
- preserve the existing bounded wait and relief-valve budgets as the primary latency envelope and
  MUST NOT rely on widening them as the primary remedy;
- preserve exact same-version semantics for any produced ready snapshot;
- preserve latest-wins supersession and cancellation when a newer same-file revision or newer save
  cycle overtakes the target;
- preserve truthful fallback to degraded paths only when bounded recovery proof is exhausted or the
  target is superseded.

#### Scenario: Still-current same-version producer leaves `parser_base_recovery` in bounded time

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the dominant exact blocker would otherwise remain `parser_base_recovery`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** runtime executes save-critical parser-base recovery for the exact target
- **THEN** the producer prioritizes only the bounded recovery work required to prove or install a
  matching parser base for that target
- **AND** the path reaches later exact work or materializes ready artifacts without falling back to
  `shadow_state` solely because `parser_base_recovery` monopolized the same-version exact path

#### Scenario: Exhausted recovery proof preserves truthful fallback

- **GIVEN** `didSave` heavy follow-up is waiting on an exact same-version producer
- **AND** bounded save-critical parser-base recovery cannot prove or install a matching parser base
- **WHEN** runtime exhausts that recovery proof
- **THEN** the system MAY fall back truthfully to the existing degraded path
- **AND** observability preserves that `parser_base_recovery` was the exhausted blocker rather than
  hiding the incident under a generic parse delay
