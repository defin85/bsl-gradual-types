## ADDED Requirements

### Requirement: Same-version `didSave` heavy follow-up MUST wake on the first matching diagnostics artifact within the bounded wait window

The system MUST treat canonical live `ready_artifacts` materialization and detached
diagnostics-ready artifact publication as two distinct bounded wake sources for the same
still-current `didSave` target whenever heavy follow-up is waiting on same-version readiness for
that target.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent same-save identity;
- prefer canonical `ready_artifacts` immediately when they are already materialized;
- during the bounded wait, race canonical `ready_artifacts` materialization against matching
  detached diagnostics-ready artifact publication for that same target;
- allow canonical `ready_artifacts` to win if live exact readiness materializes first;
- allow detached diagnostics-ready artifacts to win only while canonical live exact readiness is
  still pending for the same target;
- use a cancellation-safe or semantically equivalent restart-safe wake surface so repeated
  wait-loop restarts do not lose detached publication events;
- preserve latest-wins supersession, diagnostics-generation matching, version matching,
  `save_cycle_sequence` matching, cancellation, and truthful miss outcomes when the target is no
  longer current;
- preserve fail-closed semantics for `hover`, `definition`, `signatureHelp`, completion exact
  upgrade, and semantically equivalent interactive exact consumers until canonical live exact
  readiness completes;
- export operator-facing evidence that names which wake source won (`ready_artifacts`,
  `detached_ready_artifacts`, or a truthful miss outcome) and how long the bounded wait lasted;
- MUST NOT satisfy this requirement by widening the bounded wait budget or by treating detached
  diagnostics-ready state as canonical live exact readiness.

#### Scenario: Detached diagnostics-ready publication wins the bounded wait before canonical timeout

- **GIVEN** `didSave` heavy follow-up is waiting on a still-current same-version target
- **AND** canonical live exact `ready_artifacts` are not yet materialized for that target
- **AND** a matching detached diagnostics-ready artifact is published during the bounded wait
- **WHEN** the waiter resolves the first matching wake source
- **THEN** the heavy follow-up completes through `detached_ready_artifacts`
- **AND** it does not burn the rest of the bounded wait budget merely because canonical
  `ready_install` is still pending
- **AND** exported evidence names `detached_ready_artifacts` as the wake winner

#### Scenario: Canonical ready artifacts still win if they materialize first

- **GIVEN** `didSave` heavy follow-up is waiting on a still-current same-version target
- **AND** both canonical ready-artifact materialization and detached publication are possible for
  that target
- **WHEN** canonical live exact `ready_artifacts` materialize before any matching detached wake
- **THEN** the heavy follow-up completes through `ready_artifacts`
- **AND** detached diagnostics-ready publication, if it appears later, does not rewrite the winner
  for that wait

#### Scenario: Stale detached publication does not wake a newer still-current target

- **GIVEN** a newer same-file revision, diagnostics generation, or `save_cycle_sequence` has
  already overtaken an older waiting target
- **AND** a detached diagnostics-ready artifact is published for the older target
- **WHEN** the newer waiter evaluates the detached wake source
- **THEN** it ignores the stale detached publication
- **AND** terminal behavior remains truthful through supersession, mismatch, cancellation, or
  another bounded miss outcome
