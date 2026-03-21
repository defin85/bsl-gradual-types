## MODIFIED Requirements

### Requirement: Existing completion surfaces переносят `v9` pre-service-scope split без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v10` dispatch split в человекочитаемом виде.

Human-readable projection MUST:
- показывать dispatch-to-request-context split, если connected server возвращает `v10` payload;
- сохранять уже существующий `v9` pre-service-scope split и truthful provenance rules;
- явно деградировать на `v9`, не выдумывая `jsonrpc_dispatch_received_at_ms` и `dispatch_to_request_context_wait_ms`;
- для incident bundle не скрывать эту limitation за нейтральным `No gaps were recorded`.

#### Scenario: Panel и clipboard показывают dispatch split рядом с existing ingress facts
- **GIVEN** extension получает completion timeline `v10` с `jsonrpc_dispatch_received_at_ms`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый dispatch split рядом с existing pre-service-scope и pre-method fields
- **AND** оператор может отличить lag до `RequestContextService::call` от lag после middleware entry

#### Scenario: Incident bundle summary переносит `v10` split без guessed reconstruction
- **GIVEN** incident bundle строится по `v10` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts для dispatch split
- **AND** derived handoff не выдумывает этот split для `v9` payload

#### Scenario: Extension явно деградирует на `v9`
- **GIVEN** connected server возвращает completion timeline `v9`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v10` fields
- **AND** человекочитаемый output явно отмечает, что dispatch split unavailable by design

#### Scenario: Incident bundle не маскирует отсутствие `v10` split как отсутствие gaps
- **GIVEN** connected server возвращает completion timeline `v9`
- **WHEN** extension формирует `summary.md` для incident bundle
- **THEN** summary явно отмечает, что dispatch split unavailable by design for `contract=v9`
- **AND** summary не должен одновременно утверждать, что для этого missing split `No gaps were recorded`
