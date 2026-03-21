## ADDED Requirements
### Requirement: Existing completion surfaces переносят `v9` pre-service-scope split без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v9` pre-service-scope split в человекочитаемом виде.

Human-readable projection MUST:
- показывать `service_future_created` split, если connected server возвращает `v9` payload;
- сохранять уже существующие truthful `v8` provenance rules;
- явно деградировать на `v8`, не выдумывая `service_future_created_at_ms` и derived waits;
- для incident bundle не скрывать эту limitation за нейтральным `No gaps were recorded`.

#### Scenario: Panel и clipboard показывают pre-service-scope split рядом с existing pre-method facts
- **GIVEN** extension получает completion timeline `v9` с `service_future_created_at_ms`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый split рядом с existing pre-method fields
- **AND** оператор может отличить lag до `service_future_created` от lag после создания future

#### Scenario: Incident bundle summary переносит `v9` split без guessed reconstruction
- **GIVEN** incident bundle строится по `v9` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts для `service_future_created` split
- **AND** derived handoff не выдумывает этот split для `v8` payload

#### Scenario: Extension явно деградирует на `v8`
- **GIVEN** connected server возвращает completion timeline `v8`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v9` fields
- **AND** человекочитаемый output явно отмечает, что pre-service-scope split unavailable by design

#### Scenario: Incident bundle не маскирует отсутствие `v9` split как отсутствие gaps
- **GIVEN** connected server возвращает completion timeline `v8`
- **WHEN** extension формирует `summary.md` для incident bundle
- **THEN** summary явно отмечает, что pre-service-scope split unavailable by design for `contract=v8`
- **AND** summary не должен одновременно утверждать, что для этого missing split `No gaps were recorded`
