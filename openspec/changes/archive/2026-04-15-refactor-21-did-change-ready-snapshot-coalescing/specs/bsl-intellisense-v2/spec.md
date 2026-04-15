## MODIFIED Requirements

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)

На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и
edit chain.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить
на full parse для той же exact target revision и фиксировать причину fallback в observability.

Для same-file burst revisions система MUST оставаться latest-wins:

- obsolete intermediate same-file revisions MAY быть coalesced до parse/materialization;
- система MUST NOT materialize ready snapshot для obsolete intermediate revision, если уже известен
  newer exact same-file target;
- система MUST NOT ухудшать exactness: materialized ready snapshot по-прежнему обязан совпадать с
  latest exact target revision/text hash.

#### Scenario: same-file burst coalesces obsolete revisions before parse starts

- **GIVEN** для одного `file_id` приходят `didChange` revisions `V`, `V+1`, `V+2` в пределах одного
  burst
- **AND** older ready-snapshot work ещё не начал blocking parse для `V`
- **WHEN** runtime prepares background ready-snapshot production
- **THEN** older target revisions MAY быть coalesced away before parse starts
- **AND** blocking parse starts only for the latest exact target revision available at that moment
- **AND** obsolete intermediate revisions do not materialize ready snapshots

#### Scenario: newer exact target suppresses stale materialization after older parse finished

- **GIVEN** background ready-snapshot production already parsed exact revision `V`
- **AND** before materialization/install the same file receives newer revision `V+1`
- **WHEN** the producer re-checks latest exact target before publishing ready artifacts
- **THEN** the producer skips stale materialization for `V`
- **AND** retargets to `V+1` instead of publishing obsolete exact artifacts

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)

После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны или представлены exact still-current
coalesced producer для matching `(file_id, requested_version, text_hash)`.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- boundedly wait only when runtime can prove that the coalesced producer still matches the same
  exact requested revision/text;
- не тратить bounded wait на producer, который уже retargeted/coalesced away в пользу newer
  revision;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: didSave waits only on an exact still-current coalesced producer

- **GIVEN** `didSave` already completed same-version `save_fastlane`
- **AND** runtime sees a coalesced ready-snapshot producer whose exact target matches the same
  `(file_id, requested_version, text_hash)`
- **WHEN** heavy follow-up chooses between `ready_artifacts` and `shadow_state`
- **THEN** runtime may spend the existing bounded wait budget on that exact producer
- **AND** if the producer materializes within budget, the same save cycle may publish through exact
  `ready_artifacts`

#### Scenario: didSave skips waiting for a coalesced-away producer

- **GIVEN** `didSave` completed `save_fastlane` for revision `V`
- **AND** the file-scoped producer has already retargeted to newer revision `V+1`
- **WHEN** heavy follow-up chooses its semantic path
- **THEN** runtime does not spend bounded wait on the no-longer-exact producer for `V`
- **AND** falls back truthfully to `shadow_state` or other existing bounded fallback for `V`

## ADDED Requirements

### Requirement: Incident bundles distinguish coalesced producer churn from exact timeout (MUST)

Incident-bundle observability MUST expose low-cardinality lifecycle evidence for same-file
ready-snapshot production so operators can distinguish:

- work that was coalesced away before parse;
- work that parsed but was skipped before materialization because a newer target already existed;
- exact same-version producer wait that still timed out and forced `shadow_state` fallback.

This evidence MUST remain bounded and MUST NOT require raw logs to explain whether same-file churn
came from unnecessary worker starts or from a legitimate exact target that still lost to budget.

#### Scenario: Bundle shows coalesced churn instead of masking it as generic superseded work

- **GIVEN** a same-file burst produces several obsolete intermediate revisions before the newest
  exact target materializes
- **WHEN** an operator exports an observability incident bundle
- **THEN** the bundle distinguishes coalesced/retargeted producer outcomes from exact timeout
  outcomes
- **AND** the operator can tell whether `didSave` waited on the right exact producer or whether the
  older revisions were already coalesced away
