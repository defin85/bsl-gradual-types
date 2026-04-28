## ADDED Requirements

### Requirement: Canonical ready install MUST bound exact type-index wait after detached diagnostics-ready publication (MUST)

The system MUST bound canonical ready snapshot install after detached diagnostics-ready
publication. After same-file `didChange` or same-version `didSave` promotion publishes detached
diagnostics-ready artifacts for the current text, canonical ready snapshot install MUST either reach
exact type-index readiness inside the explicit checked-in readiness envelope or export a truthful
low-cardinality blocker before reporting materialization success.

This behavior MUST:

- keep detached diagnostics-ready publication distinct from canonical live ready snapshot install;
- preserve canonical exact gates for completion, hover, definition, signatureHelp,
  type-at-position, and semantically equivalent interactive exact consumers;
- wait only for the exact requested file version and MUST NOT install a canonical ready snapshot
  for a stale or mismatched version;
- preserve latest-wins supersession, cancellation, retarget, and latest-version-mismatch outcomes;
- expose ready-install wait elapsed time, explicit wait ceiling or deadline class, outcome, active
  requested version, type-index task phase, exact readiness boolean, current canonical ready
  snapshot version, parse snapshot metadata state, and serve-only blocked state when available;
- treat multi-second canonical materialization latency after detached diagnostics-ready publication
  as a residual unless the report proves a contract-approved blocker such as supersession,
  cancellation, retarget, latest-version mismatch, continuity loss, type-index invalidation, or
  serve-only blocked readiness;
- preserve checked-in p56 baseline comparison for canonical materialization latency, including the
  baseline p50/p95, observed p50/p95, and deltas or equivalent pass/fail evidence;
- satisfy the contract without widening existing readiness or relief budgets as the primary remedy
  and without replacing a tens-of-seconds residual with an implicit unreported wait.

#### Scenario: Detached diagnostics-ready does not hide canonical type-index wait

- **GIVEN** a same-file edit publishes detached diagnostics-ready artifacts for revision `V`
- **AND** the canonical ready snapshot is still installed for an older revision `V-1`
- **AND** exact type-index readiness for `V` is false
- **AND** a type-index precompute task for `V` is active but not ready
- **WHEN** canonical ready snapshot install waits for exact type-index readiness before
  `record_ready_parse_snapshot_v2`
- **THEN** the report records the ready-install wait elapsed time and blocker state
- **AND** representative validation treats multi-second waiting as a failure unless the blocker is
  a truthful contract-approved outcome
- **AND** detached diagnostics-ready publication alone does not satisfy canonical live ready
  install acceptance

#### Scenario: Canonical ready install succeeds after exact type-index readiness

- **GIVEN** a same-file edit targets revision `V`
- **AND** detached diagnostics-ready artifacts may already be published for diagnostics follow-up
- **WHEN** exact type-index readiness for `V` is available within the explicit checked-in readiness
  envelope
- **THEN** canonical ready snapshot install records success for revision `V`
- **AND** materialization metrics report bounded ready-install/type-index wait evidence
- **AND** interactive exact consumers continue to observe only canonical exact-ready state

### Requirement: Ready snapshot materialization metrics MUST use effective source attribution after promotion (MUST)

The system MUST attribute ready-parse-snapshot materialization metrics, phase metrics, and
lifecycle source labels to the effective target source after same-version promotion or retarget,
while preserving the original worker-start source as separate evidence.

This behavior MUST:

- record `original_source` from the worker-start target;
- record `effective_source` from the target used for final canonical ready install and
  materialization metric emission;
- record a low-cardinality promotion or retarget event when a same-version `didSave` mutates or
  promotes a running `didChange` target;
- prevent final materialization histograms from being silently attributed only to `did_change` when
  the effective target was promoted to `did_save`;
- prevent ready-snapshot phase metrics and lifecycle terminal labels from being silently attributed
  only to the worker-start source after promotion or retarget;
- refresh or otherwise derive the effective source immediately before detached diagnostics-ready
  publication, canonical ready install, lifecycle completion, materialization metric emission, and
  phase metric emission;
- preserve lifecycle and save-family identity for same-version didSave exact producers;
- avoid unbounded metric labels such as file paths, text hashes, or diagnostic text.

#### Scenario: didSave promotion updates effective materialization source

- **GIVEN** a background parse snapshot worker starts from a `didChange` target for revision `V`
- **AND** a same-version `didSave` for the same text promotes or mutates that target before final
  materialization
- **WHEN** the worker records canonical ready snapshot materialization
- **THEN** the metric uses `effective_source=did_save`
- **AND** the report still preserves `original_source=did_change`
- **AND** the report identifies the promotion event as same-version didSave promotion

#### Scenario: Pure didChange materialization remains didChange attributed

- **GIVEN** a background parse snapshot worker starts from `didChange`
- **AND** no didSave promotion, retarget, or source mutation occurs before final materialization
- **WHEN** the worker records canonical ready snapshot materialization
- **THEN** both original and effective source are reported as `did_change`
- **AND** the didChange materialization histogram remains valid evidence for a pure didChange
  canonical ready-install path

### Requirement: Representative validation MUST fail unexplained high didChange canonical materialization latency (MUST)

Representative live validation for same-file p56 save/change flows on `examples/conf_big` MUST fail
when canonical ready snapshot materialization remains high after detached diagnostics-ready
publication and the report lacks phase/source evidence that truthfully explains the delay.

Checked-in evidence for this gate MUST preserve at least:

- detached diagnostics-ready publication elapsed time and terminal outcome;
- canonical ready-install exact type-index wait elapsed time, explicit ceiling/deadline class, and
  outcome;
- original source, effective source, and promotion/retarget event;
- observed latest version and current canonical ready snapshot version;
- exact type-index readiness boolean;
- type-index precompute task phase, active requested version, and work class;
- parse snapshot metadata state and serve-only blocked state when available;
- final canonical ready snapshot source and version;
- blocker class when the target is superseded, cancelled, retargeted, latest-version mismatched,
  invalidated, or blocked for serve-only readiness.
- checked-in baseline comparison for canonical materialization p50/p95.

#### Scenario: p56 gate rejects unexplained tens-of-seconds didChange materialization

- **GIVEN** representative p56 validation publishes diagnostics follow-up through detached
  diagnostics-ready artifacts inside the bounded window
- **AND** the same report records `did_change_ready_snapshot_materialization_ms` p50 or p95 in the
  tens of seconds
- **AND** per-cycle evidence shows the latest observed version is newer than the canonical ready
  snapshot version
- **AND** exact type-index readiness is false with a type-index task still active or no parse
  snapshot metadata available
- **WHEN** the report lacks a truthful blocker or source-promotion explanation
- **THEN** representative validation fails this canonical materialization residual
- **AND** the failure is not counted as a refactor-54 detached diagnostics-ready acceptance gap

#### Scenario: p56 gate accepts bounded or truthfully classified canonical materialization

- **GIVEN** representative p56 validation publishes detached diagnostics-ready artifacts quickly
- **WHEN** canonical ready install either reaches exact type-index readiness inside the explicit
  checked-in envelope or exports a truthful blocker/source-promotion classification
- **THEN** the report records both detached and canonical timelines
- **AND** the validation accepts the sample without weakening canonical exact gates
