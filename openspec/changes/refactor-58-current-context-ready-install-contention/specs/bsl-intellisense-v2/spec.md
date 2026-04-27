## ADDED Requirements

### Requirement: Current-context and didSave ready-install contention MUST be bounded and attributable (MUST)

The system MUST keep completion handling isolated from concurrent
current-context and diagnostics readiness work, while making the non-completion
work itself bounded and attributable.

For `bsl.getCurrentContext`, representative request evidence MUST identify the
request route, generation/version, broker role where applicable, readiness wait
result, parse source, supersession or budget outcome, wall time, and final
status. Equivalent same-generation bursts MUST share bounded expensive work, and
requests for stale generations MUST stop, downgrade, or report supersession once
newer work makes their result obsolete. A current-context request MUST NOT remain
visible only as an aged completion contender without its own request outcome.

For same-file `didSave` follow-up readiness/install work, seconds-scale
`ready_install`, `snapshot_with_deps`, or `wait_for_file_version` residuals MUST
either be removed by bounded runtime behavior or exported through explicit
low-cardinality blocker buckets. A generic `ready_install` number without
lower-level attribution is not sufficient acceptance evidence when the residual
is materially larger than parse execution.

Representative live metrics or incident bundles captured for this change MUST
keep observability integrity intact:

- `intellisense_v2_observability_contract_violation_total` absent or `0`;
- invalid saturation metric violations absent or `0`;
- generic saturation and dedicated runtime-lane surfaces still bounded and
  contract-complete.

#### Scenario: Post-saturation bundle isolates readiness contention

- **GIVEN** a representative bundle captured after the saturation contract fix
  shows `observability_contract_violation_total=0`
- **AND** completion probes do not show seconds-scale client pre-send,
  transport ingress, or output handoff waits
- **WHEN** same-file `didSave` follow-up or `bsl.getCurrentContext` work shows
  seconds-scale elapsed time
- **THEN** the incident evidence classifies the residual as current-context or
  readiness/install contention, not as a UI, transport, or completion-path
  regression
- **AND** the bundle contains first-class evidence for the affected
  current-context requests or didSave readiness blockers

#### Scenario: Current-context bursts stay latest-only and bounded

- **GIVEN** multiple `bsl.getCurrentContext` requests target the same document
  generation or an equivalent ready-snapshot key
- **AND** a newer generation arrives while older requests are in flight
- **WHEN** the current-context handler processes the burst
- **THEN** at most one expensive leader performs work per equivalent key
- **AND** followers complete through a bounded shared result, budget exhaustion,
  or supersession outcome
- **AND** obsolete generation work does not continue as opaque seconds-scale
  background contention
- **AND** each request outcome is exported in current-context request evidence

#### Scenario: didSave follow-up does not hide seconds-scale ready-install waits

- **GIVEN** a same-file `didSave` follow-up has fast parse execution but
  seconds-scale readiness/install elapsed time
- **WHEN** the save timeline or incident bundle is exported
- **THEN** the residual is attributed to explicit blocker buckets such as exact
  file-version wait, exact type-index wait, runtime lane queue wait,
  `snapshot_with_deps` queue/exec wait, publish/apply lock wait, supersession,
  or unclassified residual
- **AND** validation fails if the only explanation is a generic
  seconds-scale `ready_install` value

#### Scenario: Completion remains healthy under current-context and readiness load

- **GIVEN** concurrent current-context requests and same-file diagnostics
  readiness work are in flight
- **WHEN** a completion request for the same document arrives
- **THEN** completion ingress, handler execution, and output handoff remain
  within the existing bounded completion contract
- **AND** current-context contender ages are treated as advisory concurrency
  evidence unless the completion timeline itself shows blocking
- **AND** accepting the change requires the observability contract violation
  counters to remain absent or zero
