## ADDED Requirements

### Requirement: Started same-version `didSave` exact producers MUST bound parser-base recovery before shadow-state fallback (MUST)

The system MUST NOT let a still-current same-version `didSave` exact producer that already reached lifecycle `started` terminate diagnostics heavy follow-up through `shadow_state` solely because bounded waiting timed out inside `parser_base_recovery`.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent save-family identity;
- treat `started` as a producer-owned lifecycle boundary that must progress to detached
  diagnostics-ready publication, full materialization, supersession, cancellation, failure, or
  explicit continuity loss;
- keep detached diagnostics-ready publication as the bounded success endpoint for this
  diagnostics-only follow-up path;
- preserve the existing bounded wait and relief-valve budgets as the latency envelope and MUST NOT
  be satisfied solely by widening those budgets;
- preserve truthful `shadow_state`, supersession, cancellation, failure, or continuity-loss fallback
  when a newer same-file revision or newer save cycle overtakes the target, or when the runtime can
  no longer prove the started producer still owns the exact save family;
- preserve canonical live exact readiness gates for completion, hover, definition, signatureHelp,
  type-at-position, and semantically equivalent interactive exact consumers;
- export bounded low-cardinality evidence for lifecycle at timeout and final same-family lifecycle
  after timeout or fallback.

#### Scenario: Started same-family producer reaches detached diagnostics-ready instead of shadow-state fallback

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish for revision `V`
- **AND** heavy follow-up is still targeting the same current save family
- **AND** the exact producer lifecycle for that family has reached `started`
- **AND** bounded wait would otherwise time out with `followup_ready_snapshot_timeout_leaf=parser_base_recovery`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** runtime resolves the heavy-follow-up path
- **THEN** the bounded success endpoint is detached diagnostics-ready publication or full exact
  materialization for that same save family
- **AND** the terminal diagnostics publish does not use `followup_semantic_path=shadow_state` solely
  because parser-base recovery was still in progress

#### Scenario: Truthful non-exact terminal outcome remains allowed

- **GIVEN** heavy follow-up is waiting on a started same-version `didSave` exact producer
- **AND** bounded parser-base recovery cannot progress that producer to detached diagnostics-ready
  publication within the existing envelope, or the producer no longer owns the exact save family
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY terminate through `shadow_state`, supersession, cancellation, failure, or
  continuity-loss fallback
- **AND** exported evidence preserves the truthful terminal reason rather than reporting only
  lifecycle `started`

### Requirement: Representative validation MUST fail on started parser-base timeout followed by shadow-state fallback (MUST)

Representative live or perf validation for same-file `didSave` follow-up on `examples/conf_big` MUST fail if all of the following are true in the same captured run:

- `save_fastlane` already published quickly for the same save family;
- the follow-up gate/admission path is not the dominant delay;
- the same-version exact producer lifecycle is `started`;
- bounded wait times out with `followup_ready_snapshot_timeout_leaf=parser_base_recovery`;
- the follow-up terminal semantic path is `followup_semantic_path=shadow_state`;
- semantic diagnostics query dominates the fallback branch; and
- no per-cycle final lifecycle evidence proves detached/full exact readiness, supersession,
  cancellation, failure, or continuity loss for that same save family.

Checked-in evidence for this gate MUST preserve at least:

- `requested_version`, `save_cycle_sequence`, and save-family identity;
- `followup_save_fastlane_gate_outcome` and gate/admission wait values;
- lifecycle at bounded timeout and final lifecycle after timeout/fallback;
- zero-budget, bounded-wait, and relief-valve outcomes;
- timeout phase, timeout leaf, subphase/checkpoint, and elapsed values;
- terminal semantic path and semantic query elapsed values;
- detached diagnostics-ready publication, full materialization, or truthful non-exact terminal
  reason for the same save family.

#### Scenario: Live gate rejects the 2026-04-24 started-producer parser-base residual

- **GIVEN** representative same-file `didSave` profiling on a large module
- **AND** `save_fastlane` already published the same-version first refresh
- **AND** the exact producer lifecycle reached `started`
- **AND** bounded wait timed out at `parser_base_recovery`
- **WHEN** the measured follow-up sample still publishes through `shadow_state`
- **AND** exported evidence shows query-dominated semantic work on that fallback path
- **AND** the sample lacks a truthful same-family terminal producer reason beyond `started`
- **THEN** the representative gate fails
- **AND** the regression is not treated as the already-closed `refactor-51` waiting-only producer
  admission case
