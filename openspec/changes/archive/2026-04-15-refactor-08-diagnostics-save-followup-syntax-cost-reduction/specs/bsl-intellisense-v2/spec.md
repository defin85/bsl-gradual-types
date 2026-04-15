## MODIFIED Requirements
### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- переиспользовать same-version syntax artifacts в `didSave + idle_heavy`, если их freshness
  доказана для данного save cycle;
- truthfully fall back to syntax recompute, когда reuse невозможно или stale;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker

#### Scenario: same-version syntax artifacts avoid redundant full syntax recompute
- **GIVEN** save cycle already has same-version syntax artifacts suitable for the requested version
- **WHEN** `idle_heavy` builds richer follow-up diagnostics
- **THEN** the server reuses those syntax artifacts instead of rerunning full-file syntax query as
  the primary expensive step
- **AND** the follow-up timeline stays truthful about whether syntax was reused or recomputed
