## ADDED Requirements

### Requirement: Post-refactor-58 didSave program-lowering tail MUST be bounded and attributable (MUST)

The system MUST NOT accept a seconds-scale exact `program_lowering` tail as a
generic readiness success after a clean post-refactor-58 bundle proves that
completion ingress/egress, current-context attribution, `ready_install`, and
measured `snapshot_with_deps` are not the dominant blockers for a same-version
`didSave` heavy follow-up.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash,
  save_cycle_sequence)` target, or a semantically equivalent save-family
  identity;
- preserve bounded `save_fastlane` first publish as independently user-visible;
- preserve detached diagnostics-ready artifacts as the correct diagnostics-only
  terminal endpoint without treating eventual detached-ready publication as
  sufficient when exact assembly arrives after a seconds-scale tail;
- distinguish `ready_install`, measured `snapshot_with_deps`,
  semantic diagnostics, and exact `parse_exec -> exact_ready_snapshot_assembly
  -> program_lowering` time in the request-centric save trace;
- reject or refine a generic `followup_readiness_blocker_bucket=snapshot_with_deps`
  explanation when `snapshot_with_deps_ms` is small and the dominant measured
  residual is exact program lowering;
- export program-lowering reuse outcome, rebuilt/reused lowering unit counts,
  reuse-plan source and hit flags end-to-end through backend diagnostics-save
  timeline evidence, VS Code custom request typing, incident-bundle raw JSON, and
  human-readable summary when program lowering dominates;
- treat missing program-lowering reuse evidence as a validation gap when the
  program-lowering tail is seconds-scale;
- allow a truthful required-full-rebuild reason only when exported evidence
  proves reuse was unavailable or unsafe for the exact save family, including
  reuse outcome, rebuilt/reused unit counts, reuse-plan source/hit flags, and a
  low-cardinality invalidation reason;
- allow truthful supersession, cancellation, failure, or continuity-loss reasons
  when exact assembly cannot be bounded;
- preserve canonical exact readiness for completion, hover, definition,
  signatureHelp, type-at-position, and semantically equivalent interactive exact
  consumers;
- preserve observability integrity: contract violations and invalid saturation
  metric violations MUST remain absent or zero in representative validation.

#### Scenario: Program-lowering tail is not accepted as generic snapshot-with-deps

- **GIVEN** a representative post-refactor-58 bundle has clean observability
  integrity
- **AND** completion `service_future_to_first_poll_wait_ms` and output handoff
  remain bounded
- **AND** `save_fastlane` first publish for a same-version `didSave` save family
  completes quickly
- **AND** the same follow-up has small `ready_install_ms` and small measured
  `snapshot_with_deps_ms`
- **AND** exact `parse_exec -> exact_ready_snapshot_assembly ->
  program_lowering` is seconds-scale and dominates the follow-up tail
- **WHEN** diagnostics-save timeline and incident bundle evidence are exported
- **THEN** the residual is classified as exact assembly/program-lowering
  materialization tail, or as a truthful required-full-rebuild/supersession/
  cancellation/failure/continuity-loss outcome
- **AND** representative validation does not accept the sample as merely a
  generic `snapshot_with_deps` blocker

#### Scenario: Missing lowering reuse evidence is fail-visible

- **GIVEN** same-version `didSave` heavy follow-up times out with
  `followup_ready_snapshot_timeout_leaf=program_lowering`
- **AND** program lowering is the dominant exact assembly checkpoint
- **WHEN** request-centric diagnostics-save evidence is exported
- **THEN** the backend timeline, VS Code custom request type, incident-bundle raw
  JSON, and human-readable summary include program-lowering reuse outcome,
  rebuilt/reused lowering unit counts, reuse-plan source, and reuse-plan hit flags
  when available
- **AND** if those fields are unavailable, the bundle or representative report
  records an explicit missing-evidence gap instead of treating the residual as
  accepted

### Requirement: Representative post-refactor-58 validation MUST gate didSave program-lowering tail (MUST)

Representative validation for this change MUST use a post-refactor-58 large
module save-follow-up scenario and fail when a captured same-file `didSave`
cycle has the residual shape shown by
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z`:

- installed runtime includes refactor-58 current-context and diagnostics-save
  readiness attribution surfaces;
- observability contract violations are absent or `0`;
- invalid saturation metric violations are absent or `0`;
- completion fail-closed count is `0`, and completion ingress/egress do not
  explain the incident;
- first publish is bounded;
- `ready_install` is not the dominant residual;
- measured `snapshot_with_deps_ms` is not the dominant residual;
- full follow-up publishes through `detached_ready_artifacts`;
- `timeout_phase=parse_exec` and `timeout_leaf=program_lowering` or
  semantically equivalent exact assembly evidence is present;
- exact assembly/program-lowering elapsed time is seconds-scale;
- no save-cycle-local evidence proves required full rebuild with reuse outcome,
  rebuilt/reused unit counts, reuse-plan source/hit flags, and invalidation
  reason, or proves supersession, cancellation, failure, or continuity loss; and
- lowering reuse evidence is either missing or proves a reuse miss that remains
  above the latency envelope.

Checked-in evidence for this gate MUST preserve at least:

- comparison against the `refactor-08` live report or a later checked-in
  baseline that shows the speed improvement being preserved;
- comparison against the pre-refactor-58 readiness/install bundle when relevant;
- request count and source status for completion, current-context, and
  diagnostics-save timelines;
- `requested_version`, `save_cycle_sequence`, first publish elapsed, full
  follow-up elapsed, terminal semantic path, and readiness blocker bucket;
- measured `ready_install`, `snapshot_with_deps`, semantic diagnostics,
  `parse_exec`, exact assembly, program conversion, and program lowering
  timings;
- bounded-wait and relief-valve outcomes;
- program-lowering reuse outcome, lowering unit counts, reuse-plan source and hit
  flags, plus the source/projection status proving those fields survived through
  backend timeline, VS Code custom request typing, incident-bundle raw JSON, and
  human-readable summary, or an explicit missing-evidence gap.

#### Scenario: Fresh post-refactor-58 bundle drives the next narrow change

- **GIVEN** runtime git `033ac549` or a later equivalent includes refactor-58
  current-context and readiness attribution
- **AND** the captured bundle proves the old ready-install residual is no longer
  dominant
- **AND** completion and current-context evidence stay bounded or attributable
- **WHEN** the same bundle still shows a seconds-scale `didSave` follow-up tail
  dominated by exact program lowering
- **THEN** the next acceptance gate targets program-lowering tail boundedness
  and evidence completeness
- **AND** the change is not considered complete by reclassifying the sample under
  generic `snapshot_with_deps` alone
