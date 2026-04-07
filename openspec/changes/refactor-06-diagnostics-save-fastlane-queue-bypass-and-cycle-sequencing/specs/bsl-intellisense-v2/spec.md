## MODIFIED Requirements
### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh,
инициированного `textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`,
  `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух
  save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как
  synthetic surrogate для direct save-fastlane bypass path.

#### Scenario: didSave refresh экспортируется с dedicated save-cycle identity
- **GIVEN** пользователь сохраняет документ
- **WHEN** diagnostics runtime запускает refresh для `didSave`
- **THEN** система создаёт request-centric trace этого refresh
- **AND** trace содержит monotonic `save_cycle_sequence`
- **AND** trace можно получить через dedicated diagnostics save timeline surface

#### Scenario: operator-facing ordering двух save-cycle не зависит от diagnostics_generation
- **GIVEN** документ получает два `didSave` при одном и том же `requested_version`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** система показывает distinct `save_cycle_sequence` для каждого cycle
- **AND** trace остаётся truthful даже если `diagnostics_generation` не годится как save ordering key

### Requirement: didSave save_fastlane публикует bounded same-version first refresh (MUST)
`save_fastlane` MUST давать bounded same-version first publish после `didSave`, даже если
applied-analysis snapshot ещё не готов.

Если `save_fastlane` падает в syntax-only shadow fallback, этот path MUST:

- не ждать shared bounded interactive queue как primary gating step;
- не публиковать diagnostics от older revision;
- оставаться supersession-aware для newer `didSave`.

#### Scenario: save_fastlane shadow fallback bypass-ит shared queue starvation
- **GIVEN** shared interactive blocking queue насыщена другой работой
- **AND** `didSave` first publish вынужден идти через shadow parse fallback
- **WHEN** diagnostics runtime публикует `save_fastlane` first refresh
- **THEN** first publish не тратит seconds-scale latency только на shared queue wait
- **AND** trace не маскирует bypass synthetic `blocking_queue_wait_ms`
