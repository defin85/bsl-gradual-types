## ADDED Requirements

### Requirement: Representative same-file save-follow-up MUST bound diagnostics-only semantic query residual once the exact path is stable

The system MUST reduce diagnostics-only semantic query latency on the representative same-file
`didSave` heavy follow-up family once that family already remains on current exact
`ready_artifacts`, without regressing exactness truthfulness.

This behavior MUST:

- preserve the current exact `ready_artifacts` path for supported representative same-file
  save-follow-up targets;
- preserve diagnostics-only semantic materialization for supported cases, or preserve truthful full
  fallback when parity cannot be proven;
- NOT be satisfied solely by widening upstream wait budgets or by silently shifting supported
  diagnostics-only work onto the full semantic-facts path;
- preserve operator-facing evidence that distinguishes diagnostics-only current-exact work from
  full fallback and shows where the dominant semantic residual moved.

#### Scenario: Representative family stays exact while diagnostics-only semantic query residual drops

- **GIVEN** a representative same-file save-follow-up family already publishes through current exact
  `ready_artifacts`
- **AND** diagnostics-only semantic query is the dominant remaining residual on that family
- **WHEN** the runtime executes semantic diagnostics for that representative family
- **THEN** refreshed representative evidence shows lower diagnostics-only semantic query latency
  than the checked-in `refactor-39` baseline
- **AND** the family still remains on `ready_artifacts`
- **AND** the traced semantic path remains diagnostics-only unless a truthful full fallback is
  required

#### Scenario: Unsupported optimization does not fake a latency win through silent fallback

- **GIVEN** an attempted diagnostics-only optimization cannot preserve semantic parity for the
  current exact target
- **WHEN** the runtime executes semantic diagnostics for that target
- **THEN** it preserves truthful diagnostics-only versus full-fallback attribution
- **AND** it does not claim success by silently downgrading supported work to full fallback or by
  publishing stale semantic results
