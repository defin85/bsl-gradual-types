## ADDED Requirements
### Requirement: Completion timeline `v9` сужает `transport_to_service_scope_wait_ms` до bounded pre-service-scope segments (MUST)
Authoritative `bsl.getCompletionTimeline` payload MUST поднимать contract до `v9` и MUST сохранять существующие `v8` server-edge fields, дополняя их bounded split внутри уже известного сегмента `transport_received -> service_scope_entered`.

Если payload включает новый pre-service-scope split, он MUST использовать только additive bounded поля:
- optional `service_future_created_at_ms`;
- optional `transport_to_service_future_wait_ms`;
- optional `service_future_to_scope_wait_ms`.

Если `service_future_created_at_ms` присутствует, payload MUST включать и оба derived waits, чтобы оператору не приходилось вручную вычитать timestamp'ы.

#### Scenario: Запрос тормозит до возврата `inner.call(request)`
- **GIVEN** completion request получает большой lag до момента, когда service future уже создан
- **WHEN** сервер сериализует completion timeline `v9`
- **THEN** payload содержит bounded pre-service-scope split
- **AND** `transport_to_service_future_wait_ms` показывает положительную задержку
- **AND** старые поля `transport_to_service_scope_wait_ms` и `transport_to_method_wait_ms` остаются доступны

#### Scenario: Запрос тормозит после создания service future, но до первого poll request scope
- **GIVEN** completion request уже имеет созданный service future, но ещё не вошёл в request scope
- **WHEN** сервер сериализует completion timeline `v9`
- **THEN** payload содержит положительный `service_future_to_scope_wait_ms`
- **AND** оператор может отличить этот случай от lag до возврата `inner.call(request)`

### Requirement: `v9` pre-service-scope split сохраняет trustworthy attribution semantics из `v8` (MUST)
Новый bounded split MUST не ослаблять existing `v8` integrity semantics для pre-method attribution.

Сервер MUST:
- сохранять existing `pre_method_attribution_provenance`;
- не подменять отсутствие `v9` split guessed полями;
- не добавлять free-text/high-cardinality debug fields.

#### Scenario: Connected server ещё не поддерживает `v9`
- **GIVEN** connected server возвращает completion timeline `v8`
- **WHEN** extension или operator читает authoritative payload
- **THEN** payload не выдумывает `service_future_created_at_ms`
- **AND** trustworthy provenance semantics остаются ограничены уже существующими `v8` полями
