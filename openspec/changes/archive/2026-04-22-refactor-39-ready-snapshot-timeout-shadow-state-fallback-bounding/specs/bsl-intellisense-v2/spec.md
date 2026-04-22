## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST bound terminal `shadow_state` fallback while a still-current exact producer remains in `parse_exec`

The system MUST prefer a bounded still-current exact path when `didSave` heavy follow-up is
waiting on an exact same-version producer that is still current and already inside bounded
`parse_exec`.

On the representative save-follow-up family, terminal `shadow_state` fallback MUST remain a
truthful exception rather than the steady-state outcome for that state.

This behavior MUST:

- remain bound to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)`
  target, or a semantically equivalent per-save-cycle identity;
- preserve the existing bounded wait and relief-valve budgets as the primary latency envelope;
- NOT be satisfied solely by widening those budgets instead of improving still-current producer
  continuity, proof, or promotion behavior;
- avoid repeatedly terminating the heavy follow-up on
  `wait_probe=timeout -> relief_valve=engaged_timed_out -> shadow_state` solely because the
  initial bounded wait elapsed while the same producer remained the newest valid target;
- preserve exact same-version semantics for any produced ready artifacts;
- preserve truthful supersession, cancellation, and fallback when a newer same-file revision or
  newer save cycle overtakes the current target, or when the runtime can no longer prove that the
  in-flight producer remains the bounded best candidate;
- preserve operator-facing low-cardinality evidence that distinguishes:
  - a still-current exact continuation path that remained eligible after the initial timeout;
  - a terminal `shadow_state` fallback because still-current continuation proof was exhausted;
  - truthful supersession, cancellation, or other terminal non-continuation outcomes.

#### Scenario: Still-current same-version `parse_exec` producer wins the heavy follow-up path

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish
- **AND** the heavy follow-up is waiting on a still-current exact same-version producer that is
  already inside bounded `parse_exec`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the runtime executes the representative save-follow-up policy
- **THEN** the heavy follow-up publishes through `ready_artifacts`
- **AND** `shadow_state` is not the terminal branch solely because the initial bounded wait elapsed

#### Scenario: Truthful fallback remains when the current exact target is no longer provable

- **GIVEN** the heavy follow-up exhausted its initial bounded wait on an exact same-version
  producer
- **AND** either a newer same-file revision or newer save cycle overtakes that target, or the
  runtime can no longer prove that the in-flight producer remains the bounded best candidate
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY terminate truthfully through `shadow_state` or `superseded_generation`
- **AND** the exported evidence preserves whether still-current continuation was attempted
- **AND** the exported evidence preserves why the still-current exact path was not chosen
