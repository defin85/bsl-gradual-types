## MODIFIED Requirements

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy
follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary
gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- если runtime уже знает, что exact same-version ready-snapshot task для requested revision
  находится in-flight, пробовать bounded wait за этим task before consuming `shadow_state`;
- не вводить такой bounded wait, когда exact same-version task не доказан;
- при stale, superseded, cancelled, mismatched или other-version task state немедленно уходить в
  truthful `shadow_state` или generic fallback path;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: exact same-version snapshot task in flight beats shadow fallback
- **GIVEN** `didSave` already completed same-version `save_fastlane`
- **AND** runtime знает, что exact same-version ready-snapshot task для requested revision сейчас
  in-flight
- **WHEN** heavy follow-up выбирает semantic path для того же save cycle
- **THEN** runtime сначала выполняет bounded wait за exact same-version ready-artifacts path
- **AND** only after that falls back to `shadow_state`, если snapshot всё ещё unusable
- **AND** не публикует stale diagnostics другой revision

#### Scenario: absent exact task keeps immediate truthful fallback
- **GIVEN** `didSave` completed `save_fastlane`
- **AND** runtime не видит exact same-version ready-snapshot task для requested revision
- **WHEN** heavy follow-up выбирает semantic path
- **THEN** runtime не тратит bounded wait только на speculative exact snapshot hope
- **AND** остаётся на текущем truthful fallback path (`shadow_state` или generic) для этой revision
