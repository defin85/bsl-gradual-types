## ADDED Requirements

### Requirement: Same-file `didSave` heavy follow-up MUST stop treating `shadow_state` as the steady-state terminal path once a bounded exact producer is still current

After `save_fastlane` already published the same-version first refresh, the system MUST prefer a
still-current exact same-version ready-snapshot producer strongly enough that `shadow_state`
remains a truthful fallback rather than the steady-state terminal branch for bounded
`program_lowering` workloads.

This behavior MUST:

- preserve the existing bounded wait budgets as the primary latency envelope;
- keep the still-current exact producer on the hottest path once it has already entered bounded
  `ready_snapshot_assembly` / `program_lowering`;
- avoid branch selection or same-file churn policies that repeatedly starve the best exact
  candidate while it is still the latest valid producer for the save cycle;
- preserve latest-wins supersession, cancellation, and exact same-version guarantees.

#### Scenario: Still-current bounded exact producer publishes the heavy follow-up through `ready_artifacts`

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish
- **AND** the heavy follow-up is waiting on a still-current exact same-version producer that is
  already inside bounded `program_lowering`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the representative same-file mixed profile continues under the existing bounded
  follow-up policy
- **THEN** the heavy follow-up publishes through `ready_artifacts`
- **AND** `shadow_state` is not the terminal branch for that save cycle

#### Scenario: Newer same-file target still supersedes the exact producer truthfully

- **GIVEN** the heavy follow-up is currently waiting on a bounded exact same-version producer
- **AND** a newer same-file revision or newer save cycle arrives before publish
- **WHEN** the runtime re-evaluates the still-current target
- **THEN** the older producer MAY be superseded, cancelled, or retargeted truthfully
- **AND** the system does not keep the older save cycle alive just to avoid a `shadow_state`
  fallback

### Requirement: Same-file ranged `didChange` MUST keep a parser-base-capable exact head close enough to `shadow_state`

The system MUST keep a parser-base-capable exact head close enough to the live `shadow_state` that
`ready_snapshot_lags_shadow_state` stops being the dominant steady-state explanation for
`fallback_reason=stale_parser_base` on representative large-module same-file churn profiles.

This behavior MUST:

- remain bound to the exact `(file_id, requested_version, text_hash)` target;
- prefer advancing one still-current exact head or bounded recovery/prime path over repeatedly
  spawning parse workers that are predictably retargeted during `parse_exec`;
- preserve truthful fallback when a matching parser base still cannot be proven;
- preserve latest-wins semantics and MUST NOT reuse stale parser-base state for a newer revision.

#### Scenario: Representative ranged churn advances or recovers a parser-base-capable head before defaulting to `stale_parser_base`

- **GIVEN** same-file ranged `didChange` churn has advanced `shadow_state` beyond the latest ready
  exact head
- **AND** the next ranged revision would otherwise report
  `fallback_reason=stale_parser_base` with root cause `ready_snapshot_lags_shadow_state`
- **WHEN** the runtime chooses the next exact build / recovery path for that revision
- **THEN** it first advances or recovers a parser-base-capable exact head for the newest still-current target
- **AND** the newest ranged `didChange` does not default immediately to `stale_parser_base` solely
  because the old ready head lagged behind `shadow_state`

#### Scenario: Truthful fallback remains when no matching parser base can be proven

- **GIVEN** same-file ranged churn still cannot prove a matching parser base for the newest
  still-current revision after the bounded freshness / recovery path is exhausted
- **WHEN** the runtime finalizes the parse path for that revision
- **THEN** it MAY still fall back truthfully through `stale_parser_base`
- **AND** observability preserves that the bounded freshness / recovery path was attempted and
  exhausted for the same exact target
