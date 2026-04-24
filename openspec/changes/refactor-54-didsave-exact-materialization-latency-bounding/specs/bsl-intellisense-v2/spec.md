## ADDED Requirements

### Requirement: Same-version `didSave` first publish MUST remain independently bounded while exact materialization is active (MUST)

After `textDocument/didSave`, `save_fastlane` first publish MUST remain independently bounded and
auditable even when a still-current same-version exact producer is concurrently inside
`parser_base_recovery`, `parse_exec`, `program_lowering`, or semantically equivalent exact
materialization work.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent save-family identity;
- treat first-publish latency as independently user-visible and MUST NOT hide it behind a later
  successful `idle_heavy` follow-up;
- avoid blocking syntax-only `save_fastlane` publication on multi-second exact materialization work
  unless the runtime exports a truthful first-publish blocker;
- preserve latest-wins supersession and cancellation when a newer same-file revision or newer save
  cycle overtakes the target;
- preserve diagnostics quality truthfulness for syntax-only fallback and MUST NOT publish exact
  semantic diagnostics before exactness is proven;
- export low-cardinality evidence for first-publish profile, publish kind, elapsed time, syntax
  diagnostics query elapsed time, syntax work mode, and the relevant exact materialization blocker
  when one is known.

#### Scenario: Slow syntax-only first publish is not hidden by later detached-ready success

- **GIVEN** `didSave` starts a same-version save family for revision `V`
- **AND** `save_fastlane` first publish uses syntax-only diagnostics
- **AND** a still-current same-family exact producer is also inside `parser_base_recovery`,
  `parse_exec`, or equivalent exact materialization work
- **WHEN** first publish spends multiple seconds in syntax diagnostics query or another
  first-publish blocker
- **THEN** representative validation treats that first-publish latency as a failure unless the
  runtime exports a truthful first-publish blocker allowed by the contract
- **AND** a later `idle_heavy` publish through `detached_ready_artifacts` does not erase the
  first-publish failure

#### Scenario: Fast first publish remains compatible with later exact follow-up

- **GIVEN** `didSave` starts a same-version save family for revision `V`
- **AND** exact materialization for that family is still in progress
- **WHEN** `save_fastlane` can publish syntax-only diagnostics from bounded current-document state
- **THEN** first publish completes within the existing fastlane envelope
- **AND** heavy follow-up may still wait for exact `ready_artifacts` or `detached_ready_artifacts`
  under the separate follow-up contract

### Requirement: Same-version `didSave` detached-ready follow-up MUST bound exact materialization latency (MUST)

The system MUST treat a still-current same-version `didSave` heavy follow-up as an exact
materialization latency residual when it publishes through `detached_ready_artifacts` but the same
captured cycle also shows that exact materialization exceeded the bounded wait and relief-valve
envelope due to `parser_base_recovery`, `program_lowering`, full rebuild, or semantically equivalent
producer work.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent save-family identity;
- keep detached diagnostics-ready publication as the correct diagnostics-only terminal endpoint;
- treat `detached_ready_artifacts` after bounded-wait timeout and relief-valve timeout as a latency
  residual unless the runtime exports truthful supersession, cancellation, failure, continuity
  loss, or another contract-approved non-exact reason;
- treat `program_lowering_reuse_outcome=full_rebuild` with non-zero rebuilt units and zero reused
  units as a first-class exact materialization residual on representative same-file saves;
- preserve the existing bounded wait and relief-valve budgets as the latency envelope and MUST NOT
  be satisfied solely by widening those budgets;
- preserve canonical live exact readiness gates for completion, hover, definition, signatureHelp,
  type-at-position, and semantically equivalent interactive exact consumers;
- export bounded evidence for wait outcomes, timeout phase/leaf, parser-base and program-lowering
  phase timings, reuse/rebuild counts, terminal semantic path, follow-up elapsed time, and final
  same-family lifecycle.

#### Scenario: Detached-ready terminal path is still a failure when exact full rebuild misses the envelope

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish for revision
  `V`
- **AND** heavy follow-up is still targeting the same current save family
- **AND** bounded wait times out in `parse_exec` with
  `followup_ready_snapshot_timeout_leaf=program_lowering`
- **AND** relief valve also times out
- **AND** `program_lowering_reuse_outcome=full_rebuild`
- **AND** program-lowering evidence reports non-zero rebuilt units and zero reused units
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the same save family later publishes heavy follow-up through `detached_ready_artifacts`
- **THEN** representative validation treats the sample as an exact materialization latency failure
  unless a truthful terminal reason independent of bounded-wait expiry and full-rebuild reuse miss
  is exported
- **AND** the sample is not accepted merely because the terminal semantic path avoided
  `shadow_state`

#### Scenario: Bounded detached-ready follow-up remains success

- **GIVEN** `didSave` already completed same-version `save_fastlane` first publish
- **AND** heavy follow-up waits on a still-current exact producer
- **WHEN** matching `ready_artifacts` or `detached_ready_artifacts` become available within the
  bounded wait or contract-approved relief envelope
- **THEN** heavy follow-up publishes through that exact diagnostics-ready artifact
- **AND** exported evidence identifies the winner and elapsed time without reporting an exact
  materialization latency failure

### Requirement: Representative validation MUST distinguish detached-ready correctness from materialization latency (MUST)

The system MUST make representative live or perf validation for same-file `didSave` follow-up on
`examples/conf_big` fail when a captured save family satisfies either of these residual contours:

- first-publish contour:
  `save_fastlane` syntax-only first publish takes multiple seconds, syntax diagnostics query is the
  dominant first-publish cost, and no truthful first-publish blocker explains the delay; or
- detached-ready materialization contour:
  heavy follow-up eventually publishes through `detached_ready_artifacts`, but the same cycle shows
  bounded-wait timeout, relief-valve timeout, `timeout_phase=parse_exec`,
  `timeout_leaf=program_lowering` or equivalent exact materialization blocker,
  `program_lowering_reuse_outcome=full_rebuild`, zero reused units, non-zero rebuilt units, and no
  truthful supersession, cancellation, failure, or continuity-loss reason.

Checked-in evidence for this gate MUST preserve at least:

- `requested_version`, `save_cycle_sequence`, and save-family identity;
- first publish profile, publish kind, elapsed time, syntax diagnostics query elapsed time, and
  syntax work mode;
- zero-budget, bounded-wait, and relief-valve outcomes;
- timeout phase, timeout leaf, subphase/checkpoint, and elapsed values;
- `parser_base_recovery` and `program_lowering` phase timings;
- `program_lowering_reuse_outcome`, rebuilt/reused lowering units, and reuse-plan hit flags when
  available;
- terminal semantic path, semantic diagnostics query elapsed, and follow-up publish elapsed;
- lifecycle at bounded timeout and final lifecycle after timeout/fallback.

#### Scenario: Live gate rejects the 2026-04-24 detached-ready materialization residual

- **GIVEN** representative same-file `didSave` profiling on a large module
- **AND** completion transport and output handoff are not the dominant delay
- **AND** one save cycle publishes syntax-only `save_fastlane` only after multi-second syntax query
- **OR** another save cycle publishes heavy follow-up through `detached_ready_artifacts` only after
  bounded wait timeout, relief-valve timeout, and full `program_lowering` rebuild
- **WHEN** exported evidence lacks a truthful supersession, cancellation, failure, continuity-loss,
  or first-publish blocker reason for the corresponding delay
- **THEN** the representative gate fails
- **AND** the regression is not treated as the already-closed terminal `shadow_state` fallback
  contour
