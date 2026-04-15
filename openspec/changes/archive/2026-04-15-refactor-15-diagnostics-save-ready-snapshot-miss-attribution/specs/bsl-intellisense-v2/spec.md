## MODIFIED Requirements

### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного
`textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`,
  `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy
  follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух
  save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как
  synthetic surrogate для direct save-fastlane bypass path;
- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что
  primary blocker это `apply_lag` / `wait_for_file_version`;
- публиковать canonical low-cardinality outcome для zero-budget `ready_artifacts` probe;
- публиковать canonical low-cardinality outcome для bounded-wait `ready_artifacts` probe, если
  такой probe был выполнен;
- публиковать branch-selection context, достаточный чтобы оператор видел:
  - был ли `shadow_state` доступен в момент выбора ветки;
  - существовала ли same-version ready-snapshot task и в каком canonical task state она была;
- публиковать эти новые поля через additive versioned contract, где older payload versions
  деградируют явно как `unavailable_by_design`, а не silently.

#### Scenario: zero-budget ready-snapshot miss explains why shadow-state won
- **GIVEN** `didSave` cycle already completed `save_fastlane`
- **AND** exact same-version ready parse snapshot не был выбран для `idle_heavy`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** timeline показывает explicit outcome zero-budget ready-snapshot probe
- **AND** timeline показывает, был ли доступен `shadow_state`
- **AND** timeline показывает canonical ready-snapshot task state вместо неявного `None`

#### Scenario: older timeline payload degrades explicitly
- **GIVEN** consumer читает diagnostics save timeline payload более старой contract version
- **WHEN** в этой версии ещё нет ready-snapshot miss attribution fields
- **THEN** consumer маркирует их как `unavailable_by_design`
- **AND** оператор не принимает отсутствие поля за отсутствие события
