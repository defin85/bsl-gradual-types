## ADDED Requirements

### Requirement: Pure didChange canonical ready materialization MUST stay within the checked-in p56 baseline (MUST)

The system MUST restore representative pure didChange canonical ready snapshot
materialization latency to the checked-in p56 baseline. For a still-current
non-save-cycle didChange revision, canonical ready snapshot install MUST reach
exact type-index readiness and emit successful materialization evidence within
the checked-in p56 baseline envelope.

This behavior MUST:

- apply to pure didChange targets without `save_cycle_sequence`;
- preserve canonical exact gates for completion, hover, definition,
  signatureHelp, type-at-position, and semantically equivalent exact consumers;
- use the checked-in p56 materialization baseline as the acceptance reference,
  including `p50 <= 3226ms` and `p95 <= 3329ms` unless a later approved change
  updates the baseline with evidence;
- expose non-save-cycle didChange ready-install/type-index wait elapsed time,
  ceiling/deadline class, terminal outcome, active requested version, observed
  latest version, current canonical ready snapshot version, exact readiness,
  type-index task phase, parse snapshot metadata state, and serve-only blocked
  state when available;
- treat a still-current pure didChange deadline or blocker as a failed readiness
  outcome for this baseline, not as successful materialization;
- preserve latest-wins supersession, cancellation, retarget, and
  latest-version-mismatch outcomes as non-success terminal outcomes that do not
  enter successful pure didChange materialization histograms;
- avoid satisfying the baseline by widening thresholds, weakening exact
  readiness, or counting later didSave/save-cycle blocker classifications as a
  didChange pass.

#### Scenario: Current pure didChange installs canonical ready snapshot within baseline

- **GIVEN** a pure didChange target for current revision `V`
- **AND** no same-version didSave promotion or save-cycle target owns the final
  canonical install
- **WHEN** exact type-index readiness for `V` becomes available before the
  checked-in p56 baseline is exceeded
- **THEN** canonical ready snapshot install records success for `V`
- **AND** the successful pure didChange materialization sample contributes to
  the p56 didChange baseline view
- **AND** representative validation reports `did_change_materialization_within_baseline=true`

#### Scenario: Current pure didChange blocker does not count as materialization success

- **GIVEN** a pure didChange target for current revision `V`
- **AND** exact type-index readiness for `V` is not available before the
  checked-in p56 baseline is exceeded
- **WHEN** the ready-install wait exports a deadline or blocker outcome
- **THEN** the target is recorded as a non-success ready-install blocker
- **AND** the sample does not enter successful pure didChange materialization
  histograms
- **AND** representative validation fails unless a later approved requirement
  explicitly changes the pure didChange acceptance contract

#### Scenario: Superseded didChange is excluded with a terminal reason

- **GIVEN** a pure didChange target for revision `V`
- **AND** a newer revision supersedes `V` before canonical ready install
- **WHEN** the worker exits before successful materialization
- **THEN** the report records a superseded, cancelled, retargeted, or
  latest-version-mismatch terminal reason
- **AND** the sample is excluded from successful pure didChange materialization
  latency

### Requirement: Materialization metrics MUST distinguish pure didChange success from blockers and save-cycle work (MUST)

The system MUST keep successful pure didChange canonical materialization samples
separate from classified ready-install blockers and from didSave-promoted or
save-cycle canonical install work.

This behavior MUST:

- preserve enough low-cardinality evidence to identify whether a sample is
  successful pure didChange, promoted didSave/save-cycle, or non-success blocker;
- prevent `did_change_ready_snapshot_materialization_ms` acceptance from being
  satisfied by a later save-cycle `ready_install_exact_type_index_wait` blocker;
- prevent classified non-success blockers from being emitted as successful
  materialization latency samples;
- preserve original/effective source attribution introduced by refactor-55;
- export counts for successful pure didChange samples, excluded didChange
  samples, promoted/save-cycle samples, and blocker classes in representative
  reports;
- avoid unbounded labels such as file paths, text hashes, or diagnostic text.

#### Scenario: Save-cycle blocker cannot mask pure didChange baseline failure

- **GIVEN** representative p56 validation records a later same-version
  didSave/save-cycle exact type-index blocker
- **AND** pure didChange successful materialization p50 or p95 exceeds the
  checked-in baseline
- **WHEN** validation evaluates the didChange baseline contract
- **THEN** validation fails `did_change_materialization_within_baseline`
- **AND** the later save-cycle blocker remains visible as separate evidence but
  does not make the pure didChange baseline pass

#### Scenario: Promoted save-cycle materialization is reported separately

- **GIVEN** a worker starts from didChange and is later promoted by same-version
  didSave
- **WHEN** the worker emits final canonical install or blocker evidence
- **THEN** the report preserves `original_source=did_change` and
  `effective_source=did_save`
- **AND** the sample is attributed to the promoted/save-cycle class rather than
  successful pure didChange baseline materialization

### Requirement: Representative p56 validation MUST require didChange materialization baseline success (MUST)

The system MUST make representative live validation for the p56 same-file flow
on `examples/conf_big` reject current-source runs where pure didChange canonical
ready snapshot materialization remains above the checked-in baseline.

Checked-in p56 evidence MUST include:

- successful pure didChange materialization sample count and p50/p95;
- baseline p50/p95 and observed-vs-baseline deltas;
- excluded didChange non-success count and terminal reasons;
- didSave-promoted/save-cycle sample count;
- ready-install exact type-index wait state for pure didChange targets;
- final canonical ready snapshot source and version;
- explicit `did_change_materialization_within_baseline` pass/fail value.

#### Scenario: p56 rejects high pure didChange canonical materialization

- **GIVEN** representative p56 validation reports successful pure didChange
  materialization p50 or p95 above the checked-in baseline
- **WHEN** a later save-cycle ready-install blocker is also truthfully classified
- **THEN** validation still fails the didChange materialization baseline
- **AND** the report points to pure didChange ready-install/type-index evidence
  rather than treating the save-cycle blocker as acceptance

#### Scenario: p56 accepts restored didChange baseline

- **GIVEN** representative p56 validation records successful pure didChange
  materialization samples
- **AND** observed p50 and p95 are within the checked-in baseline
- **AND** any save-cycle blocker or promotion evidence is reported separately
- **WHEN** validation evaluates the current-source run
- **THEN** `did_change_materialization_within_baseline=true`
- **AND** the run can pass without weakening canonical exact readiness
