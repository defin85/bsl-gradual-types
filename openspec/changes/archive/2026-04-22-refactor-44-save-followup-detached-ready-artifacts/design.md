## Context

The representative `p56_real_conf_big_diagnostics_representative_save_followup_bundle_live`
evidence after the parser-base work changed the diagnosis materially.

Confirmed facts:

- the still-current same-version path reaches parse/lowering quickly enough that
  `parser_base_recovery` is no longer the dominant residual;
- the remaining timeout happens in `ready_install`, not earlier exact build stages;
- current exact/head readiness for the same revision is still absent at timeout;
- `ir_singleflight_* = 0` on this representative path, so the current exact/head install is not
  being rescued by an existing shared leader;
- an experiment that published snapshot-backed live state before the exact install wait regressed
  initial `didOpen` ready publication and was reverted;
- a direct exact-prime attempt from parse snapshot did not produce a clean representative win and
  was also reverted.

Change boundary facts:

- save-critical `parser_base_recovery` boundedness and truthful parser-base fallback remain owned
  by `refactor-43-save-critical-parser-base-recovery-bounding`;
- raw-to-derived diagnostics-save timeout-leaf fidelity also remains owned by `refactor-43`;
- this change starts only after the representative residual has shifted later to `ready_install`.

So the remaining problem is narrower than "make exact faster at any cost".
`didSave` follow-up already has enough same-version work built to produce diagnostics-relevant
payload, but it is still coupled to canonical live exact install that must remain stricter for
interactive exact consumers.

## Goals / Non-Goals

- Goals:
  - let same-version `didSave` heavy follow-up consume a detached diagnostics-ready artifact before
    live exact install becomes the primary gate;
  - preserve current exact/live fail-closed rules for interactive exact consumers;
  - preserve latest-wins supersession, cancellation, and truthful fallback;
  - make the new path explicit in operator-facing evidence.
- Non-Goals:
  - publishing partial state into canonical live exact readiness earlier than today;
  - broad detached current-revision head architecture for completion or other interactive queries;
  - widening bounded wait budgets instead of reducing the coupling that currently causes the
    timeout;
  - hiding the new path under the old `ready_artifacts` label as if live exact install had already
    completed.

## Decisions

### 1. Publish a detached diagnostics-ready artifact outside live exact readiness

The system will introduce a detached diagnostics-ready artifact for the exact current
`didSave` target, keyed by `(file_id, requested_version, text_hash, save_cycle_sequence)` or a
semantically equivalent identity.

This artifact is not canonical live exact readiness.
It is a diagnostics-only read model that exists specifically so `didSave` follow-up can consume
the already-built same-version payload without waiting for the later live exact install barrier.

### 2. Produce the artifact after diagnostics-ready build, before live `ready_install`

The artifact should be published only after the exact same-version producer has built the bounded
payload needed for diagnostics follow-up, but before the later `ready_install` / type-index
publication barrier becomes the primary wait.

This keeps the detached artifact tied to real work that is already complete, rather than inventing
synthetic readiness from a partially built or guessed state.

### 3. Detached artifacts stay diagnostics-only

Interactive exact consumers must not treat detached diagnostics-ready artifacts as proof of
canonical exact readiness.

That means `hover`, `definition`, `signatureHelp`, `type-at-position`, completion exact upgrade,
and semantically equivalent exact consumers continue to require the existing live exact install.
If live exact readiness is absent, they remain fail-closed exactly as before.

### 4. Latest-wins invalidation stays strict

Detached diagnostics-ready artifacts must be invalidated or ignored when:

- a newer same-file revision arrives;
- a newer save cycle for the same revision overtakes the older one;
- the target identity no longer matches the waiting follow-up.

The new path must not permit stale detached artifacts to leak across revisions or save cycles.

### 5. Observability must name the new branch truthfully

If `didSave` follow-up succeeds through the detached artifact, operator-facing evidence must say
so explicitly instead of reporting canonical live `ready_artifacts`.

That preserves the crucial distinction between:

- full live exact readiness;
- detached diagnostics-only readiness;
- degraded `shadow_state` fallback;
- truthful supersession / cancellation.

### 5a. Detached publication can follow either timeouted or non-timeouted canonical probes

Representative live evidence already shows that `detached_ready_artifacts` can appear after more
than one canonical probe shape:

- after a truthful bounded-wait timeout with `followup_ready_snapshot_timeout_leaf=ready_install`;
- after a zero-budget miss where the detached proof is already available and no bounded-wait
  `ready` outcome is claimed.

So acceptance for this change must require truthful path labeling and preserved fail-closed
interactive semantics, but it must not overfit to the narrower claim that every detached publish
must pass through the exact same timeout sub-shape.

### 6. `refactor-44` must not absorb `refactor-43`

If representative evidence later regresses back to `parser_base_recovery` dominance or loses raw
to derived timeout-leaf fidelity again, that is a regression against `refactor-43`, not a reason
to broaden this change.

This change is specifically about decoupling diagnostics follow-up from late live exact install
after parser-base-specific work is no longer the primary bottleneck.

### 7. Representative probes are `p55` / `p56`, not `p53`

The operational representative evidence for this change is the late same-version save-followup
family that now times out in `ready_install`.

That means:

- `p55` / `p56`-style diagnostics-save followup traces are the representative acceptance surface;
- `p53_real_conf_big_exact_program_lowering_report_live` may still be useful as a legacy
  diagnostic probe if someone suspects `program_lowering` regressed again;
- but `p53` is not a required gate for this change, because this change starts only after
  `program_lowering` stopped being the dominant representative residual.

## Alternatives Considered

### 1. Widen the bounded wait or relief-valve budgets

Rejected.

The evidence already shows that waiting longer would mostly rename the same `ready_install`
residual instead of removing it.

### 2. Publish snapshot-backed live state before exact install completes

Rejected.

That experiment regressed initial `didOpen` ready publication and proved that weakening live exact
gates is unsafe.

### 3. Prime current exact install directly from parse snapshot

Rejected for this change.

The direct prime experiment did not produce a clean representative win and does not address the
core coupling problem by itself.

### 4. Fold this into the general detached current-revision head proposal

Rejected for now.

`refactor-current-revision-head-detached-snapshot` is broader and completion-oriented.
The current incident is narrower, diagnostics-save-specific, and already evidence-localized.

## Validation Strategy

- Add a backend regression where same-version `didSave` follow-up reaches diagnostics-ready state
  but live exact install is still pending, and prove the path completes through detached artifacts
  instead of terminal `shadow_state`.
- Add a paired regression proving that interactive exact consumers still remain fail-closed until
  canonical live exact readiness completes.
- Refresh representative live evidence for the `ready_install`-dominated save-followup family and
  confirm that the terminal branch changes without regressing `didOpen`/same-version readiness.
- Use the late save-followup family (`p55` / `p56`-style traces or semantically equivalent
  evidence) as the representative acceptance surface; keep `p53` only as optional supporting
  diagnostics.
- Preserve the assumption that `parser_base_recovery` and derived timeout-leaf fidelity are still
  covered by `refactor-43` rather than silently re-implementing them here.
- Run strict OpenSpec validation before handoff.
