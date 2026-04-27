## ADDED Requirements

### Requirement: didSave exact lowering reuse continuity MUST be explicit and bounded (MUST)

For a same-file `didSave` heavy follow-up, exact ready-snapshot assembly MUST
not rely solely on opportunistic parser AST cache residency to decide whether
program lowering can be reused. When a prior ready snapshot, didChange parse
snapshot, same-text ready snapshot, or semantically equivalent save-family source
is available and validates against the target file/version/text, the system MUST
derive or reuse a lowering reuse seed from that source before falling back to a
full program-lowering rebuild.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash,
  save_cycle_sequence)` target, or a semantically equivalent save-family
  identity;
- validate seed compatibility by text hash, base version, changed ranges,
  parser-tree compatibility, or an equivalent fail-closed check before reuse;
- allow the existing parser AST cache as a fast seed source without making cache
  residency the only acceptance path;
- prefer deterministic seed selection when multiple seed sources are available;
- retain at least one compatible seed for the still-current save family until
  terminal publication, supersession, cancellation, failure, or an explicit
  bounded-retention eviction with a recorded reason;
- prevent bounded retention from becoming an accepted steady-state excuse for
  immediate full rebuilds in the representative large-module save scenario;
- preserve bounded `save_fastlane` first publish as independently user-visible;
- preserve canonical exact readiness for completion, hover, definition,
  signatureHelp, type-at-position, and semantically equivalent interactive exact
  consumers;
- preserve detached diagnostics-ready artifacts as a diagnostics-only endpoint,
  without treating eventual detached publication as sufficient when exact
  assembly performs an unproved seconds-scale full rebuild;
- export the selected lowering reuse seed source or reuse-plan build source in
  request-centric diagnostics-save evidence;
- treat `full_rebuild` with `reuse_plan_build_source=null` as acceptable only
  when a low-cardinality required-full-rebuild, supersession, cancellation,
  failure, unsafe-seed, or continuity-loss reason is exported for the same trace;
- preserve program-lowering reuse outcome, rebuilt/reused lowering unit counts,
  reuse-plan source/hit flags, seed source, candidate count, eviction reason, and
  failure reason end-to-end through backend diagnostics-save timeline evidence,
  VS Code custom request typing, incident-bundle raw JSON, and human-readable
  summary when program lowering dominates; and
- preserve observability integrity: contract violations and invalid saturation
  metric violations MUST remain absent or zero in representative validation.

#### Scenario: Later same-file save does not silently lose lowering reuse

- **GIVEN** a large same-file save sequence has a prior follow-up that proves
  successful lowering reuse for the file family
- **AND** a later same-file `didSave` follow-up reaches exact
  `program_lowering`
- **AND** a compatible ready snapshot, didChange parse snapshot, same-text ready
  snapshot, or equivalent save-family source is available
- **AND** the save family has not reached a terminal, superseded, cancelled, or
  failed state
- **WHEN** exact ready-snapshot assembly builds the program-lowering reuse plan
- **THEN** the reuse seed source is selected deterministically and exported
- **AND** the follow-up does not silently fall back to `full_rebuild` because an
  opportunistic AST cache lookup missed
- **AND** if the seed was evicted by bounded retention, the trace records the
  eviction reason and the validation treats normal steady-state eviction as a
  failure for this scenario

#### Scenario: Required full rebuild is explicit

- **GIVEN** a same-file `didSave` heavy follow-up has
  `followup_ready_snapshot_timeout_leaf=program_lowering`
- **AND** program lowering is the dominant exact assembly checkpoint
- **AND** lowering reuse cannot be safely derived from any valid seed source
- **WHEN** request-centric diagnostics-save evidence is exported
- **THEN** the trace includes `full_rebuild` with rebuilt/reused unit counts
- **AND** the trace includes a low-cardinality reason proving why reuse was
  unavailable or unsafe
- **AND** if the reason is `seed_evicted` or equivalent, the trace also exposes
  whether eviction was caused by terminal cleanup, supersession, cancellation,
  failure, or bounded capacity pressure
- **AND** missing seed source with no reason is reported as a validation gap

### Requirement: Representative post-refactor-59 validation MUST gate lowering reuse continuity (MUST)

Representative validation for this change MUST use a post-refactor-59 large
module save-follow-up scenario and fail when a captured same-file `didSave`
cycle has the residual shape shown by
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T11-07-23Z`:

- installed runtime includes `refactor-59` program-lowering tail classification
  and current-context same-text ready-snapshot follow-up behavior;
- observability contract violations are absent or `0`;
- invalid saturation metric violations are absent or `0`;
- completion fallback/stale counters are `0`, and completion ingress/egress do
  not explain the incident;
- at least one same-file save cycle proves successful lowering reuse with a
  concrete source and high reused-unit count;
- a later same-file save cycle reaches `program_lowering_tail`;
- first publish remains bounded;
- measured ready-install, output handoff, client pre-send, and generic
  transport waits are not the dominant residual;
- exact assembly/program-lowering elapsed time is seconds-scale;
- `reuse_outcome=full_rebuild` rebuilds nearly all lowering units; and
- no save-cycle-local evidence proves required full rebuild with seed source,
  reuse-plan build source, hit flags, unit counts, and a required-full-rebuild
  or continuity-loss reason; and
- no evidence proves that seed eviction was a justified bounded-retention event
  rather than normal steady-state loss of the active save-family seed.

Checked-in evidence for this gate MUST preserve at least:

- runtime git identity and bundle path;
- comparison against the previous `2026-04-27T08-39-19Z` post-refactor-58/59
  baseline;
- completion request count, p95 duration, fallback/stale counters, and
  client/transport/handoff timing envelope;
- current-context route/status summary so concurrent broker parses remain
  visible but are not misclassified as the primary didSave tail cause;
- diagnostics-save `requested_version`, `save_cycle_sequence`, first publish
  elapsed, full follow-up elapsed, terminal semantic path, and readiness blocker
  bucket;
- measured `snapshot_with_deps`, semantic diagnostics, `parse_exec`, exact
  assembly, program conversion, and program lowering timings;
- bounded-wait and relief-valve outcomes;
- program-lowering reuse outcome, lowering unit counts, reuse-plan source/hit
  flags, selected seed source, seed candidate count, eviction reason, and
  failure/required-full-rebuild reason.

#### Scenario: The 2026-04-27T11:07 bundle drives a continuity fix

- **GIVEN** runtime git `5691e618` includes the refactor-59 classifier and
  projection fixes
- **AND** the captured bundle has one save cycle with
  `reuse_plan_build_source=borrowed`, `2088` reused units, and `0` rebuilt units
- **AND** a later save cycle has `program_lowering_tail`, `full_rebuild`,
  `0` reused units, `2088` rebuilt units, `take_if_unique_hit=false`,
  `borrowed_cache_hit=false`, and no build source
- **WHEN** the change is validated
- **THEN** the later cycle is fixed by a valid save-family seed or rejected with
  a truthful required-full-rebuild/continuity-loss reason
- **AND** a bounded-retention eviction reason is accepted only if it proves
  terminal cleanup, supersession, cancellation, failure, or capacity pressure
  outside the representative steady-state path
- **AND** the change is not considered complete by classification or
  instrumentation-only improvements.
