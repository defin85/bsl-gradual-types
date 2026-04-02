## ADDED Requirements

### Requirement: Existing completion surfaces различают ingress и query-body dominance без invented findings (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST использовать authoritative server stages для различения ingress bottleneck и query-body bottleneck.

Human-readable projection MUST:
- строиться только из bounded authoritative fields/stages и локальных bounded status markers;
- не публиковать verdict `adapter_before_dispatch_dominant`, если authoritative `dominant_stage` или visible `stages` показывают dominance внутри `query_bundle*`;
- переносить `query_bundle` dominance в человекочитаемом виде для panel, clipboard и incident summary, если connected server возвращает `v20` payload;
- явно деградировать на `v19`, не выдумывая detailed `query_bundle_pool_wait`, `query_bundle_ir_query` или equivalent split.

#### Scenario: Panel и clipboard не обвиняют adapter ingress при dominant query-body stage
- **GIVEN** extension получает completion timeline `v20`, где `adapter_to_dispatch_wait_ms` положителен, но authoritative `dominant_stage` находится в `query_bundle*`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** human-readable output не публикует `adapter_before_dispatch_dominant`
- **AND** output показывает truthful query-body dominance рядом с existing ingress facts

#### Scenario: Incident bundle summary переносит query-body root cause без guessed reconstruction
- **GIVEN** incident bundle строится по `v20` payload с detailed `query_bundle` stages
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded query-body stage facts и derived verdict
- **AND** summary не заменяет их guessed ingress bottleneck

#### Scenario: Extension явно деградирует на `v19`
- **GIVEN** connected server возвращает completion timeline `v19`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает detailed `query_bundle` split
- **AND** человекочитаемый output явно отмечает, что truthful query-body breakdown unavailable by design for `contract=v19`
