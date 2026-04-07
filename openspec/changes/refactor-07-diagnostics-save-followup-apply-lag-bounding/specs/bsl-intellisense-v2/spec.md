## MODIFIED Requirements
### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh,
инициированного `textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`,
  `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что
  primary blocker это `apply_lag` / `wait_for_file_version`.

#### Scenario: timeline объясняет stalled heavy follow-up request-centric причиной
- **GIVEN** `didSave` cycle уже дал `save_fastlane` first publish
- **AND** richer heavy follow-up ещё не published
- **WHEN** оператор читает diagnostics save timeline
- **THEN** trace показывает request-centric follow-up wait reason
- **AND** оператор может отличить apply-lag от semantic-work pending

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker
