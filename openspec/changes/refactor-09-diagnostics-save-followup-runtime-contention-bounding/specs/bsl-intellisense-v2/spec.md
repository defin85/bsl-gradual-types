## MODIFIED Requirements
### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного `textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`, `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как synthetic surrogate для direct save-fastlane bypass path;
- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что primary blocker это `apply_lag` / `wait_for_file_version`;
- отдельно публиковать request-centric follow-up runtime contention facts, когда сервер уже знает их из authoritative seams, включая runtime queue wait, writer/apply execution contention, `wait_for_file_version`, semantic work и publish wait;
- не скрывать seconds-scale terminal или in-flight follow-up tail за одним только `elapsed_ms`, если authoritative runtime/apply breakdown уже наблюдаем.

#### Scenario: didSave refresh экспортируется с dedicated save-cycle identity
- **GIVEN** пользователь сохраняет документ
- **WHEN** diagnostics runtime запускает refresh для `didSave`
- **THEN** система создаёт request-centric trace этого refresh
- **AND** trace содержит monotonic `save_cycle_sequence`
- **AND** trace можно получить через dedicated diagnostics save timeline surface
- **AND** trace не требует реконструкции из aggregate metrics

#### Scenario: operator-facing ordering двух save-cycle не зависит от diagnostics_generation
- **GIVEN** документ получает два `didSave` при одном и том же `requested_version`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** система показывает distinct `save_cycle_sequence` для каждого cycle
- **AND** trace остаётся truthful даже если `diagnostics_generation` не годится как save ordering key

#### Scenario: timeline объясняет stalled heavy follow-up request-centric причиной
- **GIVEN** `didSave` cycle уже дал `save_fastlane` first publish
- **AND** richer heavy follow-up ещё не published
- **WHEN** оператор читает diagnostics save timeline
- **THEN** trace показывает request-centric follow-up wait reason
- **AND** оператор может отличить runtime contention, apply-lag и semantic-work pending

#### Scenario: terminal heavy follow-up публикует runtime/apply breakdown без hidden tail
- **GIVEN** `didSave` cycle уже завершил `idle_heavy` follow-up
- **WHEN** оператор читает diagnostics save timeline
- **THEN** trace показывает отдельные follow-up facts для observed runtime/apply contention buckets
- **AND** seconds-scale tail не остаётся только в `elapsed_ms`, если authoritative seams уже были наблюдаемы

#### Scenario: fastlane fallback публикует blocking queue wait отдельно от syntax query
- **GIVEN** `save_fastlane` first publish идёт через bounded blocking fallback path
- **WHEN** trace экспортируется в diagnostics save timeline
- **THEN** queue wait перед parse фиксируется отдельно от `syntax_diagnostics_query_ms`
- **AND** оператор может отличить queue wait от actual syntax query work

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- переиспользовать same-version syntax artifacts в `didSave + idle_heavy`, если их freshness доказана для данного save cycle;
- truthfully fall back to syntax recompute, когда reuse невозможно или stale;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles;
- после bounded same-version first publish не оставлять unrelated shared interactive/runtime backlog default seconds-scale blocker для heavy follow-up;
- если runtime/apply contention всё же остаётся blocker'ом, публиковать этот blocker отдельно в request-centric trace вместо generic residual label.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker

#### Scenario: same-version syntax artifacts avoid redundant full syntax recompute
- **GIVEN** save cycle already has same-version syntax artifacts suitable for the requested version
- **WHEN** `idle_heavy` builds richer follow-up diagnostics
- **THEN** the server reuses those syntax artifacts instead of rerunning full-file syntax query as the primary expensive step
- **AND** the follow-up timeline stays truthful about whether syntax was reused or recomputed

#### Scenario: unrelated interactive backlog does not dominate post-fastlane heavy follow-up
- **GIVEN** `didSave` cycle already delivered bounded same-version `save_fastlane` first publish
- **AND** same-version follow-up inputs are already ready
- **AND** shared interactive/runtime queue is saturated by unrelated work
- **WHEN** `idle_heavy` continues the same save cycle
- **THEN** unrelated shared interactive backlog is not the default seconds-scale primary gate for this follow-up
- **AND** if runtime/apply contention still remains, the trace publishes that blocker explicitly
