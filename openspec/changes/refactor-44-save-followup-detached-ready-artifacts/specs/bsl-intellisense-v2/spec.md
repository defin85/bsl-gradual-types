## ADDED Requirements

### Requirement: Same-version `didSave` follow-up MUST use detached diagnostics-ready artifacts without weakening live exact gates

The system MUST allow same-version `didSave` heavy follow-up to complete from a detached
diagnostics-ready artifact when bounded exact work has already produced the diagnostics-ready
payload for the still-current target but canonical live exact readiness for that same target is
still blocked on `ready_install`, type-index publication, or semantically equivalent live install
work.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent same-save identity;
- publish the detached artifact outside canonical live current-revision exact readiness and outside
  APIs that interactive exact consumers treat as proof of exact readiness;
- keep the detached artifact bounded to diagnostics-save follow-up, request-centric incident
  bundle export, or semantically equivalent diagnostics-only consumers;
- allow `didSave` follow-up to prefer the detached artifact over terminal `shadow_state` fallback
  when the target remains still-current and the detached artifact is already materialized;
- preserve exact same-version semantics, latest-wins supersession, cancellation, and truthful
  fallback when a newer same-file revision or newer save cycle overtakes the target or detached
  proof is exhausted;
- preserve fail-closed semantics for `hover`, `definition`, `signatureHelp`, `type-at-position`,
  completion exact upgrade, and semantically equivalent interactive exact consumers until
  canonical live exact readiness completes;
- preserve operator-facing evidence that distinguishes detached diagnostics-ready consumption from
  canonical live `ready_artifacts`, degraded `shadow_state`, and superseded outcomes;
- MUST NOT satisfy this requirement by early-publishing snapshot-backed live exact state,
  `SetFileWithSnapshot`, or semantically equivalent partial install that makes diagnostics-ready
  state look like canonical current-revision exact readiness.

#### Scenario: `didSave` follow-up uses detached diagnostics-ready artifacts while live install is still pending

- **GIVEN** a same-version exact producer already built the bounded diagnostics-ready payload for a
  `didSave` target
- **AND** canonical live exact readiness for that target is still blocked on `ready_install`,
  type-index publication, or semantically equivalent live install work
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** `didSave` heavy follow-up resolves the still-current target
- **THEN** the follow-up completes through the detached diagnostics-ready artifact
- **AND** it does not keep live exact install as the primary gate for that diagnostics-only path
- **AND** exported evidence identifies detached diagnostics-ready consumption rather than terminal
  `shadow_state`

#### Scenario: Interactive exact consumers remain fail-closed until canonical live readiness exists

- **GIVEN** a detached diagnostics-ready artifact already exists for revision `V`
- **AND** canonical live exact readiness for revision `V` is still unavailable
- **WHEN** the IDE requests `hover`, `definition`, `signatureHelp`, `type-at-position`, or
  semantically equivalent exact behavior for revision `V`
- **THEN** the request does not treat the detached artifact as canonical exact truth
- **AND** the existing live exact-readiness / fail-closed policy remains in force

#### Scenario: Superseded same-file target does not leak detached diagnostics artifacts

- **GIVEN** a detached diagnostics-ready artifact exists for an older same-file revision or older
  `save_cycle_sequence`
- **WHEN** a newer same-file revision or newer save cycle overtakes that target
- **THEN** the older detached artifact is not consumed as the answer for the newer target
- **AND** terminal disposition remains truthful through supersession, cancellation, or another
  bounded fallback outcome
