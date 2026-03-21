## MODIFIED Requirements

### Requirement: Existing completion surfaces переносят `v9` pre-service-scope split без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v11` service-future first-poll / first-wake split в человекочитаемом виде.

Human-readable projection MUST:
- показывать first-poll / first-wake split, если connected server возвращает `v11` payload;
- сохранять уже существующие `v10` dispatch split, `v9` pre-service-scope split и truthful provenance rules;
- явно деградировать на `v10`, не выдумывая `service_future_first_poll_entered_at_ms`, `service_future_first_poll_outcome`, `service_future_first_wake_scheduled_at_ms` и `first_poll_to_first_wake_wait_ms`;
- для incident bundle не скрывать эту limitation за нейтральным `No gaps were recorded`.

#### Scenario: Panel и clipboard показывают first-poll / first-wake split рядом с existing ingress facts
- **GIVEN** extension получает completion timeline `v11` с bounded first-poll facts
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый split рядом с existing dispatch, pre-service-scope и pre-method fields
- **AND** оператор может отличить lag до первого poll future от lag после первого `Pending`

#### Scenario: Incident bundle summary переносит `v11` split без guessed reconstruction
- **GIVEN** incident bundle строится по `v11` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts для first-poll / first-wake split
- **AND** derived handoff не выдумывает этот split для `v10` payload

#### Scenario: Extension явно деградирует на `v10`
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v11` fields
- **AND** человекочитаемый output явно отмечает, что first poll / wake split unavailable by design

#### Scenario: Incident bundle не маскирует отсутствие `v11` split как отсутствие gaps
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension формирует `summary.md` для incident bundle
- **THEN** summary явно отмечает, что first poll / wake split unavailable by design for `contract=v10`
- **AND** summary не должен одновременно утверждать, что для этого missing split `No gaps were recorded`
