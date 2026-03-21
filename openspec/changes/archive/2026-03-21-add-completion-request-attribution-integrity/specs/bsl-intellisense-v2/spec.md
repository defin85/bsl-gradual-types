## ADDED Requirements
### Requirement: Completion timeline `v8` публикует trustworthy pre-method attribution provenance (MUST)
Authoritative `bsl.getCompletionTimeline` payload MUST поднимать contract до `v8`, если он переносит pre-method attribution provenance.

Если payload включает bounded pre-method facts (`service_scope_entered_at_ms`, `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`), он MUST также включать bounded provenance для этих фактов. Provenance vocabulary MUST оставаться low-cardinality и MUST различать как минимум:
- same-request authoritative attribution;
- best-effort fallback attribution.

Payload MUST NOT выдавать best-effort fallback за доказанный same-request pre-method факт.

#### Scenario: Overlapping completion на одной позиции не получает чужой authoritative ingress
- **GIVEN** два completion request пересекаются на одном и том же `uri + position`
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** trace не маркирует pre-method attribution как same-request authoritative, если provenance не доказан для этого `request_id`
- **AND** payload не маскирует best-effort fallback под strong ingress факт

#### Scenario: Request-bound attribution сохранён через service handoff
- **GIVEN** completion request сохраняет свой request context до consumer path
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** payload включает bounded provenance same-request authoritative attribution
- **AND** оператор может доверять pre-method split как факту для этого `request_id`

#### Scenario: Provenance недоступен
- **GIVEN** completion trace не может доказать provenance pre-method attribution
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** payload явно деградирует до bounded fallback/unavailable semantics
- **AND** не выдумывает strong same-request attribution

### Requirement: Pre-method attribution integrity остаётся bounded и side-effect-safe (MUST)
Integrity instrumentation для pre-method attribution MUST:
- не менять completion semantics;
- не добавлять новый unbounded лог-канал;
- использовать только bounded fields и bounded vocabulary;
- fail-open для самого completion response и fail-closed для attribution confidence.

#### Scenario: Timeline не может сохранить request-bound provenance
- **GIVEN** completion response всё ещё может быть построен, но request-bound provenance для pre-method attribution потерян
- **WHEN** timeline trace формируется
- **THEN** completion response пользователю остаётся прежним
- **AND** timeline понижает confidence или опускает strong attribution
- **AND** payload не заменяет missing provenance guessed полями
