## ADDED Requirements

### Requirement: Same-version `didSave` producers MUST bound program-lowering rebuild before shadow fallback (MUST)

The system MUST NOT let a still-current same-version `didSave` exact producer terminate diagnostics
heavy follow-up through `shadow_state` solely because bounded waiting timed out inside
`parse_exec` / `exact_ready_snapshot_assembly` / `program_lowering` after a full program-lowering
rebuild.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target, or
  a semantically equivalent save-family identity;
- distinguish waiting/parser-base residuals from rebuild-dominated `program_lowering` residuals;
- treat `program_lowering_reuse_outcome=full_rebuild` with same-family later detached-ready
  publication as a producer boundedness failure, not a normal terminal `shadow_state` outcome;
- preserve the existing bounded wait and relief-valve budgets as the latency envelope and MUST NOT
  be satisfied solely by widening those budgets;
- preserve detached diagnostics-ready publication as the bounded success endpoint for diagnostics
  follow-up;
- preserve truthful `shadow_state`, supersession, cancellation, failure, or continuity-loss fallback
  when a newer same-file revision or newer save cycle overtakes the target, or when the runtime can
  no longer prove the exact producer still owns the save family;
- NOT treat bounded-wait expiry, `program_lowering_reuse_outcome=full_rebuild`, or missing reuse
  alone as a truthful non-exact terminal reason while final same-family lifecycle later proves
  detached diagnostics-ready publication or full materialization;
- preserve canonical live exact readiness gates for completion, hover, definition, signatureHelp,
  type-at-position, and semantically equivalent interactive exact consumers;
- export bounded low-cardinality evidence for program-lowering reuse outcome, rebuilt/reused units,
  bounded-wait winner, terminal semantic path, and final same-family lifecycle.

#### Scenario: Program-lowering reuse miss does not end a still-current save family through shadow fallback

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish for revision
  `V`
- **AND** heavy follow-up is still targeting the same current save family
- **AND** bounded wait times out in `parse_exec` with
  `followup_ready_snapshot_timeout_leaf=program_lowering`
- **AND** program-lowering evidence reports `program_lowering_reuse_outcome=full_rebuild`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the same save family later reaches detached diagnostics-ready publication or full exact
  materialization
- **THEN** the runtime treats the sample as a boundedness failure unless heavy follow-up already used
  detached diagnostics-ready or exported a truthful non-exact terminal reason
- **AND** the terminal diagnostics publish does not use `followup_semantic_path=shadow_state` solely
  because program lowering was still rebuilding

#### Scenario: Truthful non-exact terminal outcome remains allowed only with ownership evidence

- **GIVEN** heavy follow-up is waiting on a same-version `didSave` exact producer
- **AND** bounded program-lowering reuse cannot progress that producer to detached diagnostics-ready
  publication within the existing envelope
- **AND** the runtime can prove that the producer no longer owns the exact save family, was
  superseded, was cancelled, failed, or lost continuity for a reason other than bounded-wait expiry
  or full-rebuild reuse miss alone
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY terminate through `shadow_state`, supersession, cancellation, failure, or
  continuity-loss fallback
- **AND** exported evidence preserves the truthful terminal reason rather than reporting only a
  rebuild timeout

### Requirement: Representative validation MUST fail on program-lowering rebuild followed by shadow fallback (MUST)

Representative live or perf validation for same-file `didSave` follow-up on `examples/conf_big` MUST
fail if all of the following are true in the same captured run:

- `save_fastlane` already published quickly for the same save family;
- the follow-up gate/admission path is not the dominant delay;
- bounded wait times out with `followup_ready_snapshot_timeout_phase=parse_exec`;
- `followup_ready_snapshot_timeout_leaf=program_lowering`;
- `program_lowering_reuse_outcome=full_rebuild`;
- program-lowering rebuilt units are non-zero and reused units are zero, or equivalent evidence shows
  a full rebuild instead of bounded reuse;
- the follow-up terminal semantic path is `followup_semantic_path=shadow_state`;
- the same captured save family later reports `detached_diagnostics_ready_published` or
  `fully_materialized`; and
- no per-cycle evidence proves supersession, cancellation, failure, continuity loss, or another
  truthful non-exact terminal reason that is independent of bounded-wait expiry and full-rebuild
  reuse miss.

Checked-in evidence for this gate MUST preserve at least:

- `requested_version`, `save_cycle_sequence`, and save-family identity;
- `followup_save_fastlane_gate_outcome` and gate/admission wait values;
- zero-budget, bounded-wait, and relief-valve outcomes;
- timeout phase, timeout leaf, subphase/checkpoint, and elapsed values;
- `program_lowering_reuse_outcome`, rebuilt/reused lowering units, and reuse-plan hit flags when
  available;
- terminal semantic path and semantic query elapsed values;
- lifecycle at bounded timeout and final lifecycle after timeout/fallback.

#### Scenario: Live gate rejects the 2026-04-24 program-lowering rebuild residual

- **GIVEN** representative same-file `didSave` profiling on a large module
- **AND** `save_fastlane` already published the same-version first refresh
- **AND** bounded wait timed out at `program_lowering`
- **AND** program-lowering evidence shows a full rebuild rather than bounded reuse
- **WHEN** the measured follow-up sample still publishes through `shadow_state`
- **AND** exported final lifecycle later proves detached diagnostics-ready or full materialization for
  the same save family
- **AND** the sample lacks a truthful non-exact terminal reason independent of bounded-wait expiry
  and full-rebuild reuse miss
- **THEN** the representative gate fails
- **AND** the regression is not treated as the already-closed waiting or parser-base residual
