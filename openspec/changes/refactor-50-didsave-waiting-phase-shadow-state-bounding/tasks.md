## Status

This change is superseded for implementation by
`refactor-51-didsave-exact-producer-lane-bounding`.

Keep this checklist as the diagnostic/audit trail for the waiting-phase `shadow_state` incident.
Do not mark the implementation or behavioral validation tasks complete in this change unless the
same producer-side fix and representative fail gate are also completed through `refactor-51`.

## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement that same-version `didSave` heavy follow-up
      MUST bound waiting-phase `shadow_state` fallback once `save_fastlane` already published and
      the exact same-version producer remains still current.
- [x] 1.2 Define representative acceptance that fails if `examples/conf_big` still lands on
      `followup_semantic_path=shadow_state` with `timeout_phase=waiting` and query-dominated
      semantic work while the same request family later materializes exact ready state.

## 2. Design

- [x] 2.1 Describe the exact save-target identity, waiting-only producer state, and admissible
      non-shadow outcomes for the same save cycle.
- [x] 2.2 Describe truthful fallback behavior when newer revision/save-cycle supersession or lost
      continuity proof makes the still-current exact path no longer safe to prefer.
- [x] 2.3 Describe the representative live/perf evidence and worst-outlier correlation slice for
      waiting-phase `shadow_state` fallback, including semantic-query cost after fallback.

## 3. Implementation

- [ ] 3.1 Introduce a runtime change that prevents waiting-only same-version `didSave` heavy
      follow-up from defaulting to expensive terminal `shadow_state` semantic publication while the
      exact producer is still provably current. Owned by `refactor-51`.
- [ ] 3.2 Keep detached diagnostics-ready consumption, canonical live exact semantics, and
      operator-facing timeout/wait-state attribution truthful on top of the new path. Owned by
      `refactor-51`.
- [ ] 3.3 Add regressions for waiting-phase same-version follow-up, truthful fallback semantics,
      and representative diagnostics-save timeline evidence. Owned by `refactor-51`.

## 4. Validation

- [ ] 4.1 Run targeted backend/runtime/diagnostics-save regressions for the new waiting-phase
      follow-up path and preserved fail-closed semantics. Owned by `refactor-51`.
- [ ] 4.2 Run representative live/perf validation for the waiting-phase `didSave` same-version
      gate on `examples/conf_big`. Owned by `refactor-51`.
- [x] 4.3 Run `openspec validate refactor-50-didsave-waiting-phase-shadow-state-bounding --strict --no-interactive`.
