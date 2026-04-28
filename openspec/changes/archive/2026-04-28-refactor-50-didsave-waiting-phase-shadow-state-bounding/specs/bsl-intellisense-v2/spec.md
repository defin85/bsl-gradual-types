## ADDED Requirements

### Requirement: Same-version `didSave` heavy follow-up MUST bound waiting-phase `shadow_state` fallback once `save_fastlane` already published (MUST)

After successful same-version `save_fastlane` first publish, the system MUST NOT treat expensive
terminal `shadow_state` semantic publication as the steady-state heavy-follow-up outcome solely
because the still-current exact same-version producer remained in `waiting` before `parse_exec`.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent save-cycle identity;
- preserve the existing bounded wait budgets as the primary latency envelope and MUST NOT be
  satisfied solely by widening those budgets;
- distinguish waiting-only exact delay from rebuild-stage `parse_exec/program_lowering` delay;
- avoid defaulting the same save cycle to query-dominated `shadow_state` semantic publication while
  the exact same-version producer remains provably current and later exact materialization for that
  same family is still possible;
- preserve latest-wins supersession, cancellation, and truthful fallback when a newer same-file
  revision or newer save cycle overtakes the target, or when the runtime can no longer prove that
  the waiting exact producer remains the best still-current candidate;
- preserve canonical live exact semantics for interactive exact consumers.

#### Scenario: Still-current waiting-only exact producer does not end the save cycle through expensive shadow-state publish

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish for revision `V`
- **AND** heavy follow-up is still targeting the same current save cycle
- **AND** the exact producer is `in_flight_same_version`
- **AND** bounded wait attribution times out in `waiting` rather than in rebuild-stage `parse_exec`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the runtime resolves the representative heavy-follow-up path for that save cycle
- **THEN** the save cycle does not default to terminal full semantic publication through
  `shadow_state` solely because the exact producer stayed in `waiting`
- **AND** `shadow_state` is not the steady-state terminal branch for that still-current target

#### Scenario: Truthful fallback remains when the waiting exact target is no longer provable

- **GIVEN** heavy follow-up exhausted its initial bounded wait on an exact same-version producer
- **AND** either a newer same-file revision or newer save cycle overtakes that target, or the
  runtime can no longer prove that the waiting exact producer remains the best still-current
  candidate
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY still terminate truthfully through `shadow_state`, `superseded_generation`, or
  another truthful non-exact outcome
- **AND** exported evidence preserves why the still-current exact path was not chosen

### Requirement: Representative save-followup validation fails on waiting-phase query-dominated `shadow_state` fallback (MUST)

Representative live/perf validation for same-file `didSave` follow-up on `examples/conf_big` MUST
fail if the same-version saved revision still reaches `followup_semantic_path=shadow_state` under
all of the following conditions:

- `save_fastlane` already published quickly for the same save family;
- `followup_ready_snapshot_task_state=in_flight_same_version` remains true for the exact path;
- bounded wait times out with `followup_ready_snapshot_timeout_phase=waiting`;
- semantic query on the fallback path dominates the heavy-follow-up wall time; and
- the same run still shows later same-family exact ready-snapshot materialization rather than true
  supersession.

Checked-in evidence for this gate MUST preserve at least:

- `requested_version` and `save_cycle_sequence`;
- terminal semantic path;
- zero-budget and bounded-wait probe outcomes;
- timeout phase and relief-valve outcome;
- `semantic_diagnostics_query_ms` and, when available, the semantic materialization path;
- same-family exact ready-snapshot materialization evidence for the captured run.

#### Scenario: Live gate fails when waiting-phase timeout still ends in expensive shadow-state semantic work

- **GIVEN** representative same-file `didSave` profiling on a large module
- **AND** `save_fastlane` already published the same-version first refresh
- **AND** the exact same-version producer remains current but times out in `waiting`
- **WHEN** the measured follow-up sample still publishes through `shadow_state`
- **AND** exported evidence shows query-dominated semantic work on that fallback path
- **AND** the same captured run still materializes same-family exact ready state later
- **THEN** the representative gate fails
- **AND** the regression is not treated as the already-closed rebuild-dominated residual
