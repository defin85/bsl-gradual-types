## Context

The current p56 representative flow has two relevant phases:

1. a pure didChange stage that installs the first canonical ready snapshot;
2. a later same-version didSave/save-cycle follow-up that can publish detached
   diagnostics-ready artifacts quickly while canonical ready install waits for
   exact type-index readiness.

Refactor-55 fixed the second phase by exporting a bounded, truthful
ready-install exact type-index blocker. The remaining failing contract is the
first phase: the final histogram still reports pure didChange ready-snapshot
materialization around 40 seconds against the checked-in p56 baseline around
3.3 seconds.

The code boundary that matters is still the canonical install path:

```text
wait_for_exact_type_index_before_ready_install_v2
record_ready_parse_snapshot_v2
record_intellisense_v2_ready_parse_snapshot_materialization
```

Refactor-55 deliberately passed a bounded wait only when
`target.save_cycle_sequence.is_some()`. Pure didChange targets still have no
local ceiling in that wait and still emit the success materialization histogram
only after the canonical exact-ready path completes.

## Goals

- Make pure didChange canonical ready materialization satisfy the checked-in p56
  baseline again.
- Expose non-save-cycle didChange ready-install/type-index wait state with the
  same low-cardinality evidence as refactor-55 save-cycle waits.
- Ensure `did_change_ready_snapshot_materialization_ms` represents successful
  pure didChange canonical installs, not classified blockers or promoted
  save-cycle work.
- Make p56 fail when `did_change_materialization_within_baseline=false`.
- Preserve exactness for canonical live consumers.

## Non-Goals

- Do not turn detached diagnostics-ready artifacts into canonical exact state.
- Do not treat a deadline/blocker as success for a still-current pure didChange
  revision.
- Do not absorb the residual by increasing baseline constants.
- Do not change unrelated refactor-54/refactor-55 acceptance contours.

## Decision

### 1. Treat pure didChange baseline as its own acceptance gate

Refactor-56 must stop using later save-cycle
`ready_install_exact_type_index_wait_contract_approved_count` as a pass condition
for high `did_change_ready_snapshot_materialization_ms`.

For representative p56, success requires the pure didChange materialization
histogram to satisfy the checked-in baseline:

```text
p50 <= 3226ms
p95 <= 3329ms
```

Superseded, cancelled, retargeted, or latest-version-mismatch didChange targets
may be classified as non-success terminal outcomes, but those outcomes must not
be counted as successful didChange materialization samples.

### 2. Instrument non-save-cycle didChange ready-install wait

The same probe shape added for save-cycle waits should cover pure didChange
canonical install:

- elapsed wait and ceiling/deadline class;
- active requested version;
- observed latest version;
- current canonical ready snapshot version;
- exact readiness boolean;
- type-index task phase and work class;
- parse snapshot metadata state;
- serve-only blocked state when available;
- terminal outcome.

This keeps the residual actionable if the root fix does not immediately make
the p56 baseline pass.

### 3. Fix the root wait, not the threshold

Implementation should first determine why the pure didChange stage waits around
40 seconds for exact type-index readiness. The expected fix is in scheduling,
promotion, or readiness handoff around current-revision type-index precompute
and canonical ready install.

Acceptable implementation outcomes:

- exact type-index readiness for the pure didChange revision is available before
  the checked-in p56 baseline is exceeded; or
- the target is no longer current and is exported as a non-success terminal
  outcome that does not enter successful materialization histograms.

Unacceptable outcomes:

- raising the p56 materialization baseline;
- reporting the sample as successful materialization after a classified
  deadline;
- accepting high pure didChange latency because the later save-cycle blocker was
  classified.

### 4. Keep metric semantics explicit

The report should separate at least these classes:

- successful pure didChange canonical materialization;
- didSave-promoted/save-cycle canonical install and blocker state;
- non-success didChange ready-install blockers;
- excluded samples with exclusion reason.

The existing histogram name may remain for compatibility, but p56 acceptance
must be based on an explicit successful-pure-didChange view or equivalent
low-cardinality report fields.

## Risks

### Risk: exact type-index work is legitimately slow on large files

Mitigation: keep exact gates and expose blocker evidence, but do not count the
blocked target as a successful didChange materialization. The representative
p56 acceptance still requires the current pure didChange path to meet baseline.

### Risk: splitting metrics changes downstream dashboards

Mitigation: preserve existing series when possible and add explicit report
fields for successful, promoted, and excluded samples before changing public
metric names.

### Risk: a bounded didChange wait could hide missing readiness work

Mitigation: p56 must fail on current pure didChange deadline/blocker outcomes.
The deadline is diagnostic evidence, not acceptance for this change.
