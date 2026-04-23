## ADDED Requirements

### Requirement: The system MUST treat same-version didSave exact producers as dedicated save-critical producers through detached diagnostics-ready publication

The system MUST treat the corresponding exact producer as a first-class save-critical producer after `save_fastlane` already published the same-version first refresh for a save family, rather than as generic interactive or background work.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent save-family identity;
- assign the producer a dedicated admission lane and CPU-budget tier, or a semantically equivalent
  arbitration boundary, distinct from generic interactive exact requests and generic background
  diagnostics work;
- prove that the selected CPU-budget acquisition logic actually honors that producer boundary for
  the worker's CPU class; a lane label that is ignored by the budget implementation MUST NOT count
  as the dedicated tier;
- expose producer lifecycle progress that distinguishes at least admitted or started,
  detached diagnostics-ready published, fully materialized, superseded or cancelled, and failed;
- expose that lifecycle for the save family itself rather than only retagging one mutable per-file
  worker with promotion flags after the consumer has already exhausted its bounded wait;
- treat detached diagnostics-ready publication as the bounded success endpoint for heavy follow-up
  on that still-current save family;
- preserve truthful supersession and fallback when a newer same-file revision or newer save cycle
  overtakes the producer, or when the runtime can no longer prove that the producer remains the
  best still-current exact candidate;
- preserve canonical live exact semantics for interactive exact consumers.

#### Scenario: Still-current save family reaches detached exact readiness without defaulting to waiting-only shadow-state terminal publish

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish for revision
  `V`
- **AND** heavy follow-up is still targeting the same current save family
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the same-version `didSave` exact producer is admitted on its save-critical path
- **THEN** the bounded heavy-follow-up success endpoint is detached diagnostics-ready publication
  for that same save family
- **AND** the save cycle does not default to terminal full semantic publication through
  `shadow_state` solely because the producer was still waiting to start generic exact work

#### Scenario: Truthful non-exact fallback remains when producer continuity is lost

- **GIVEN** heavy follow-up exhausted its initial bounded wait on a same-version `didSave` exact
  producer
- **AND** either a newer same-file revision or newer save cycle overtakes that producer, or the
  runtime can no longer prove that it remains the best still-current exact candidate
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY still terminate truthfully through `shadow_state`, `superseded_generation`, or
  another truthful non-exact outcome
- **AND** exported evidence preserves why detached exact readiness was not chosen

### Requirement: Representative validation MUST fail when a still-current same-version save family remains producer-queue-bound and falls back before detached-ready publish

Representative live or perf validation for same-file `didSave` follow-up on `examples/conf_big` MUST fail if all of the following are true in the same captured run:

- `save_fastlane` already published quickly for the same save family;
- the same-version exact producer remains still current for that save family;
- bounded wait times out in `waiting` before detached diagnostics-ready publication for that save
  family;
- heavy follow-up still terminates through `followup_semantic_path=shadow_state`; and
- semantic query on that fallback branch dominates the heavy-follow-up wall time; and
- the same run later shows detached or fully materialized exact readiness for that same save
  family rather than true supersession.

Checked-in evidence for this gate MUST preserve at least:

- `requested_version` and `save_cycle_sequence`;
- producer task state or lifecycle evidence for the same save family;
- zero-budget and bounded-wait probe outcomes;
- timeout phase and terminal semantic path;
- detached diagnostics-ready publication evidence, when it later appears;
- `semantic_diagnostics_query_ms` for the fallback terminal branch;
- explicit same-family exact readiness evidence that distinguishes later detached-ready or fully
  materialized exact readiness from true supersession.

#### Scenario: Live gate fails when the producer never becomes the bounded winner for the still-current save family

- **GIVEN** representative same-file `didSave` profiling on a large module
- **AND** `save_fastlane` already published the same-version first refresh
- **AND** the same save family remains still current
- **WHEN** the exact producer stays queue-bound or waiting-bound past the bounded follow-up window
- **AND** heavy follow-up therefore terminates through `shadow_state`
- **AND** exported evidence shows query-dominated semantic work on that fallback branch
- **AND** the same captured run later shows detached or fully materialized exact readiness for that
  same save family
- **THEN** the representative gate fails
- **AND** the regression is not treated as a truthful steady-state waiting outcome
