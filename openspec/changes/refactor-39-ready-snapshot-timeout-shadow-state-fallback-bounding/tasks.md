## 1. Same-version timeout/fallback contract

- [x] 1.1 Define the `didSave` heavy follow-up contract for a still-current same-version
      `ready_snapshot` producer that is already inside bounded `parse_exec`.
- [x] 1.2 Rework the save-critical producer policy so the representative follow-up path stops
      defaulting to `engaged_timed_out -> shadow_state` while that producer remains the newest
      valid target for the save cycle.
- [x] 1.3 Preserve truthful supersession, cancellation, and fallback when a newer same-file
      revision or newer save cycle overtakes the current producer, or when bounded continuation
      proof is absent.

## 2. Observability and incident proof

- [x] 2.1 Export low-cardinality evidence that distinguishes a still-current timeout/continuation
      path from a terminal `shadow_state` fallback on the `didSave` heavy follow-up timeline.
- [x] 2.2 Refresh the representative `conf_big` live incident bundle against the
      `2026-04-17T14:06:03Z` baseline and record `ready_artifacts` vs `shadow_state` incidence for
      the heavy follow-up path.

## 3. Regressions

- [x] 3.1 Add targeted backend/runtime regressions covering a still-current same-version producer
      inside `parse_exec`, heavy follow-up exact publish through `ready_artifacts`, and reduced
      representative `shadow_state` fallback incidence.
- [x] 3.2 Add regressions proving that truthful `superseded_generation` or `shadow_state`
      termination still happens when a newer target overtakes the current producer.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime validation for the new timeout/fallback contract and the
      representative live repro.
- [x] 4.2 Run `openspec validate refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding --strict --no-interactive`.
