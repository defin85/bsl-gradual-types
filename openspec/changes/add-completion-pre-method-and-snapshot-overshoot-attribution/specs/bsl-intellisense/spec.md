## ADDED Requirements
### Requirement: Existing completion surfaces переносят `v7` pre-method и snapshot overshoot facts без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить новые `v7` authoritative facts в человекочитаемом виде и MUST явно деградировать на `v6`, не реконструируя отсутствующие поля эвристикой.

Human-readable projection MUST:
- показывать pre-method split отдельно от уже существующих `transport_to_method_wait_ms` / `transport_to_handler_wait_ms`;
- показывать bounded `snapshot_with_deps_timeout_runtime`, если он доступен;
- явно указывать, что `v7` fields unavailable by design, если bundle построен по `v6` payload.

#### Scenario: Panel и clipboard показывают новый pre-method split
- **GIVEN** extension получает authoritative completion timeline `v7` с bounded pre-method split
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает отдельные fact lines для pre-method split
- **AND** оператору не нужно открывать raw JSON, чтобы увидеть этот split

#### Scenario: Incident bundle summary показывает snapshot overshoot attribution
- **GIVEN** incident bundle построен по `v7` payload, где `prepare_timeout` содержит `snapshot_with_deps_timeout_runtime`
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request-centric summary переносит этот bounded fact в derived handoff
- **AND** summary не заменяет его guessed причиной

#### Scenario: Extension явно деградирует на `v6`
- **GIVEN** connected server возвращает completion timeline `v6`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `service_scope_*` или `snapshot_with_deps_timeout_runtime`
- **AND** человекочитаемый output явно отмечает отсутствие `v7` attribution fields
