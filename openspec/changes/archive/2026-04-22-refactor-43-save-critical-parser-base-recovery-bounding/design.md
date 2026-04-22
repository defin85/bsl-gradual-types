## Context

The incident bundle captured at `2026-04-19T14:34:41.582Z` on `0.4.155` / `fa87144a` no longer
supports a UI-first or generic parse-bucket diagnosis.

Authoritative bundle facts:

- completion hot path stays relatively small: the hottest server trace is `304ms`;
- `didSave` heavy follow-up publishes at `10246ms` and `10321ms`;
- both follow-up cycles remain `in_flight_same_version`, exhaust bounded wait plus relief valve,
  and terminate through `shadow_state`;
- raw diagnostics-save traces identify
  `followup_ready_snapshot_timeout_phase=parse_exec`,
  `followup_ready_snapshot_timeout_leaf=parser_base_recovery`,
  and `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=parser_base_recovery`;
- cumulative metrics show `ready_parse_snapshot_materialization_ms origin=did_change`
  regressed to `p50/p95=4938/5103ms`;
- the same process also shows downstream `current_context parse_source=parser_coordinator`
  at `parse_ms p50/p95=6194/10241` and `wall_ms p50/p95=9212/10342`.

This is materially worse than the accepted representative evidence refreshed earlier on
`2026-04-19`, where `didChange` ready-snapshot materialization on the same family was in the
`234/267ms` range and representative still-current cycles stayed on `ready_artifacts`.

There is also a separate observability fidelity defect:

- authoritative `raw/diagnostics_save_timeline.json` contains
  `followup_ready_snapshot_timeout_leaf=parser_base_recovery`;
- derived `incident.json` drops that leaf while still preserving the neighboring timeout phase and
  core-build timeout checkpoint fields.

So the runtime regression and the handoff/export drift belong to the same incident class:
operators can already see the decisive leaf in raw server data, but the representative same-file
path both regressed at runtime and lost fidelity in the derived bundle layer.

## Goals / Non-Goals

- Goals:
  - bound save-critical `parser_base_recovery` for still-current same-version exact producers;
  - restore representative same-file `didChange` materialization and `didSave` follow-up away from
    `shadow_state` fallback caused by that recovery stage;
  - make the healed path visible in downstream current-context evidence on the same family;
  - preserve authoritative timeout-leaf facts in derived incident-bundle outputs.
- Non-Goals:
  - widening wait or relief budgets instead of reducing the runtime bottleneck;
  - reclassifying the same slow work under another label without actually reducing or subdividing
    it truthfully;
  - redesigning the current-context broker before the ready-snapshot regression is addressed;
  - treating extension/UI latency as the main suspect without new contradictory evidence.

## Decisions

### 1. Treat `parser_base_recovery` as the next save-critical branch

Earlier work moved the bottleneck from opaque `parse_exec` into explicit checkpoints.
The new bundle says the next live residual is not "generic parse" and not "later assembly";
it is `parser_base_recovery`.

The change therefore targets the minimum work needed to prove or install a matching parser base for
the exact target before later tree-build and artifact work proceeds.

The runtime target is the still-current same-version background exact producer that `didSave`
follow-up promotes and waits on. The change MUST NOT "fix" this incident by building a parallel
didSave-only semantic branch that merely bypasses the lagging producer while leaving the producer
stuck in `parser_base_recovery`.

### 2. Keep the current bounded envelope

The bundle already proves that bounded wait and relief valve both fire truthfully and still time
out. Waiting longer would only rename the same incident.

The fix must reduce or restructure the work inside the existing bounded envelope, not widen it.

### 3. Keep current-context as downstream evidence, not primary root cause

`bsl.getCurrentContext` clearly suffers in this bundle, but the evidence points to a ready-snapshot
availability failure first and a parser-coordinator fallback second.

So the primary runtime target remains save-critical parser-base recovery. Current-context is an
important validation surface for whether the exact same-version path became available in time
again.

### 4. Fix derived incident fidelity in the same change

The authoritative raw trace already contains the decisive leaf. Losing it in derived
`incident.json` weakens request-centric handoff exactly when operators need the compact bundle most.

This export drift should be fixed together with the runtime change so the same incident class is
fully diagnosable without manually diffing raw and derived attachments.

## Root Cause Note

Current evidence says the representative regression is not caused by duplicate same-version worker
creation or by the downstream `shadow_state` branch itself.

`didSave` heavy follow-up already promotes the still-current same-version exact producer and waits
on that producer when the `(file_id, requested_version, text_hash)` target still matches.
The remaining steady-state stall comes earlier, while the exact producer is still attributed to
`parser_base_recovery`.

The current parser-base recovery path primes the tree cache through
`ParserCoordinator::prime_tree_cache_from_source_with_cancellation(...)`.
That helper does the tree-only parse needed to prove/install a matching parser base, but it also
performs optional AST priming for exact-lowering reuse before the stage can advance.

That optional AST priming is reuse preparation for later exact work, not the minimal proof/install
work required to leave `parser_base_recovery`.
So a still-current save-critical producer can spend representative bounded time inside the same
checkpoint even after it already has enough information to resume later exact work.

The runtime rework therefore needs to keep the promoted same-version exact producer on the exact
path while making `parser_base_recovery` stop after bounded parser-base proof/install work instead
of monopolizing the checkpoint with optional cache enrichment that belongs to later exact phases.

Update after the seed-based reuse attempt on `2026-04-20`:

- representative live evidence no longer points to `parser_base_recovery` as the dominant residual;
- `p56_real_conf_big_diagnostics_representative_save_followup_bundle_live` now reproduces
  `followup_ready_snapshot_parse_exec_ms=241ms`,
  `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms=9ms`, and
  `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=224ms`,
  but the same still-current cycle still falls back through `shadow_state` because
  `followup_ready_snapshot_dominant_phase=ready_install`,
  `followup_ready_snapshot_ready_install_ms=4005ms`, and
  `followup_ready_snapshot_timeout_leaf=ready_install`;
- the bounded same-version wait now times out after the parse-stage fix because exact-ready
  publication remains blocked later, not because parser-base recovery still monopolizes the exact
  path;
- an experiment that installed `SetFileWithSnapshot` into `analysis_v2` before the ready-install
  wait caused `didOpen` same-version ready publication to stall and was reverted, so the next
  runtime step should target `ready_install` / type-index gating without regressing initial
  ready-snapshot materialization.

## Boundary with `refactor-44`

Once representative evidence moved the dominant residual from `parser_base_recovery` to
`ready_install`, the remaining diagnostics-save remediation stopped fitting the original scope of
this change.

From that point forward the split is intentional:

- `refactor-43-save-critical-parser-base-recovery-bounding` owns save-critical
  `parser_base_recovery` boundedness, its truthful fallback contract, and the derived
  incident-bundle timeout-leaf fidelity work;
- `refactor-44-save-followup-detached-ready-artifacts` owns the later
  `ready_install` / type-index publication decoupling for `didSave` diagnostics follow-up via a
  detached diagnostics-ready artifact.

This change therefore must not be widened to "fix" the later residual by weakening canonical live
exact readiness. Symmetrically, `refactor-44` must not reopen parser-base dominance or the raw to
derived timeout-leaf fidelity regression already scoped here.

## Alternatives Considered

### 1. Revisit VS Code UI / transport first

Rejected.

The fresh bundle shows completion traces in the low hundreds of milliseconds and client/transport
seams in the low tens of milliseconds. That does not explain the `10s` diagnostics-save incident.

### 2. Widen bounded wait or relief-valve budgets

Rejected.

That would only keep the same slow `parser_base_recovery` path alive longer without restoring the
representative exact path.

### 3. Fix only the incident-bundle export drift

Rejected.

Preserving `timeout_leaf` is necessary, but it would only improve diagnosis. The runtime regression
would remain.

### 4. Start by redesigning current-context broker behavior

Rejected for now.

Current-context pressure looks downstream of the exact ready-snapshot miss. If restoring same-file
exact readiness removes most parser-coordinator fallback on the representative family, a broker
redesign is unnecessary for this incident class.

## Validation Strategy

- Add backend/runtime regressions that exercise still-current save-critical `parser_base_recovery`,
  exhausted recovery proof, and truthful fallback.
- Add extension incident-bundle regressions that prove authoritative timeout-leaf fields survive
  into derived `incident.json` and `summary.md`.
- Refresh representative evidence against the `2026-04-19T14:34:41.582Z` bundle and compare at
  least:
  - `ready_parse_snapshot_materialization_ms origin=did_change`;
  - `didSave` terminal path incidence (`ready_artifacts` vs `shadow_state`);
  - parser-base recovery timeout dominance;
  - same-family current-context parse source / wall cost.
- If the dominant residual is no longer `parser_base_recovery`, treat the later
  `ready_install` / detached-artifact path as `refactor-44` evidence instead of reclassifying it
  as unfinished parser-base work inside this change.
- Treat "exhausted recovery proof" as bounded failure to match/install the parser base or to leave
  `parser_base_recovery` for a later exact checkpoint within the existing envelope, not as mere
  passage of wall time inside the same checkpoint.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- Representative same-file save-follow-up no longer falls back to `shadow_state` solely because
  `parser_base_recovery` monopolized the same-version exact path.
- Representative `didChange` ready-snapshot materialization moves materially back toward the
  accepted sub-second envelope instead of remaining in the current multi-second regression band.
- Derived incident outputs preserve authoritative diagnostics-save timeout-leaf facts on supported
  contracts.
- Exact same-version semantics, latest-wins supersession, and truthful fallback behavior remain
  fail-closed.
