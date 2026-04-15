## MODIFIED Requirements

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles;
- различать случай, где `apply_lag` наблюдается до появления usable exact ready artifacts, от
  случая, где exact ready artifacts уже доказаны, но follow-up publish всё ещё не завершён;
- не оставлять final operator-facing attribution на generic `apply_lag`, если exact same-version
  ready artifacts уже current и usable для follow-up publish.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual
  blocker

#### Scenario: exact ready artifacts publish-ятся без blind wait на writer apply
- **GIVEN** для текущего `(file_id, requested_version, text_hash)` runtime уже доказал exact
  same-version ready artifacts
- **AND** writer-owned applied version всё ещё lag-ает
- **WHEN** `didSave` idle-heavy follow-up завершает выбор semantic path
- **THEN** система публикует follow-up через `ready_artifacts`
- **AND** operator-facing trace не оставляет `apply_lag` final blocker label для этого cycle

#### Scenario: apply_lag остаётся truthful только пока exact ready artifacts ещё не доказаны
- **GIVEN** `didSave` heavy follow-up ещё не имеет current exact ready artifacts
- **AND** writer-owned apply действительно остаётся primary blocker
- **WHEN** diagnostics save timeline экспортируется во время этого stall
- **THEN** trace MAY по-прежнему показывать `apply_lag`
- **AND** relief valve skip reason остаётся truthful
- **AND** система не делает speculative publish через stale или non-exact artifacts
