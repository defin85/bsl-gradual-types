## MODIFIED Requirements
### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного `textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `diagnostics_generation`, `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, diagnostics_generation)`;
- явно публиковать optional bounded blocking queue wait для fastlane fallback path, если first publish ждал bounded blocking permit.

#### Scenario: didSave refresh экспортируется как bounded authoritative trace
- **GIVEN** пользователь сохраняет документ
- **WHEN** diagnostics runtime запускает refresh для `didSave`
- **THEN** система создаёт request-centric trace этого refresh
- **AND** trace можно получить через dedicated diagnostics save timeline surface
- **AND** trace не требует реконструкции из aggregate metrics

#### Scenario: late result не resurrect-ит duplicate trace после terminal archive
- **GIVEN** save refresh cycle уже достиг terminal outcome и был заархивирован
- **WHEN** поздний `save_fastlane` или `idle_heavy` result для того же `(requested_version, diagnostics_generation)` приходит после этого
- **THEN** diagnostics save timeline не создаёт новый trace identity
- **AND** уже archived trace остаётся единственным authoritative trace для этого cycle

#### Scenario: fastlane fallback публикует blocking queue wait отдельно от syntax query
- **GIVEN** `save_fastlane` first publish идёт через bounded blocking fallback path
- **WHEN** trace экспортируется в diagnostics save timeline
- **THEN** queue wait перед parse фиксируется отдельно от `syntax_diagnostics_query_ms`
- **AND** оператор может отличить queue wait от actual syntax query work

### Requirement: save_fastlane и idle_heavy группируются в один didSave refresh cycle (MUST)
Если один `didSave` запускает сначала `save_fastlane`, а затем `idle_heavy`, система MUST экспортировать их как
части одного save refresh cycle, а не как два несвязанных trace.

Trace MUST:

- явно различать `first_publish_profile` и optional `followup_profile`;
- сохранять порядок publish событий внутри одного cycle;
- не позволять follow-up другого `didSave` быть ошибочно приписанным предыдущему cycle.

#### Scenario: fastlane и heavy follow-up видны как один refresh cycle
- **GIVEN** для одного `didSave` сначала публикуется `save_fastlane`, а затем `idle_heavy`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** он видит один save refresh cycle
- **AND** внутри него first publish и follow-up отображаются отдельно, но с общим cycle identity

#### Scenario: Новый didSave не прилипает к предыдущему refresh cycle
- **GIVEN** для документа идут два последовательных `didSave`
- **WHEN** follow-up publish второго save завершается позже первого
- **THEN** diagnostics save timeline не смешивает publish события разных save cycle
- **AND** каждый cycle остаётся request-centric и truthful для своей `version/generation`
