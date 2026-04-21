## 1. Contract

- [x] 1.1 Define the dual-artifact wait contract for same-version `didSave` heavy follow-up:
      canonical `ready_artifacts` and detached diagnostics-ready artifacts are two distinct wake
      sources for the same still-current save target.
- [x] 1.2 Define the observability contract so incident bundles expose the winning wake source and
      bounded wait outcome truthfully without weakening existing ready-snapshot phase attribution.

## 2. Implementation

- [x] 2.1 Add a first-class detached-artifact publication signal for the current save target that
      is safe to wait on inside the bounded save-followup wait loop.
- [x] 2.2 Rework the same-version `didSave` bounded wait so it races canonical ready-artifact
      materialization against matching detached-artifact publication for the same target identity,
      instead of checking detached artifacts only after canonical timeout / miss.
- [x] 2.3 Preserve canonical winner priority, latest-wins supersession, cancellation, generation
      mismatch, version mismatch, and stale `save_cycle_sequence` rejection when detached wakeups
      arrive for a non-current target.
- [x] 2.4 Extend diagnostics-save telemetry / incident-bundle projection with explicit
      dual-artifact wait winner attribution and bounded wait elapsed fields.

## 3. Regressions and evidence

- [x] 3.1 Add a backend regression proving detached diagnostics-ready publication during the
      bounded wait wakes `didSave` heavy follow-up before timeout-sized canonical wait exhaustion.
- [x] 3.2 Add paired regressions proving canonical `ready_artifacts` still win when they
      materialize first, and stale detached publications do not wake a newer target.
- [x] 3.3 Refresh representative live evidence for the `p55` / `p56` save-followup family and
      capture at least one authoritative sample where `detached_ready_artifacts` wins as the
      bounded wait result without weakening interactive exact fail-closed semantics.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime/incident-bundle regressions for dual-artifact wait winner
      selection and preserved fail-closed exact behavior.
- [x] 4.2 Run `openspec validate refactor-46-save-followup-dual-artifact-wait --strict --no-interactive`.
