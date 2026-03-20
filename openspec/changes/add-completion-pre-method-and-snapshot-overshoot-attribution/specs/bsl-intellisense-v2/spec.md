## ADDED Requirements
### Requirement: Completion timeline v7 сужает `server_before_method_entry` до bounded pre-method segments (MUST)
Authoritative `bsl.getCompletionTimeline` payload MUST поднимать contract до `v7` и MUST сохранять существующие server-edge fields, дополняя их bounded pre-method split без free-text логов.

Если payload включает новый pre-method split, он MUST использовать только additive bounded поля:
- optional `service_scope_entered_at_ms`;
- optional `transport_to_service_scope_wait_ms`;
- optional `service_scope_to_method_wait_ms`.

Если `service_scope_entered_at_ms` присутствует, payload MUST включать и оба derived waits, чтобы оператору не приходилось вручную вычитать timestamp'ы.

#### Scenario: Запрос задерживается до первого poll service future
- **GIVEN** completion request получает большой lag до начала service future
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload содержит bounded pre-method split
- **AND** `transport_to_service_scope_wait_ms` показывает положительную задержку
- **AND** старые поля `transport_to_method_wait_ms` и `transport_to_handler_wait_ms` остаются доступны

#### Scenario: Запрос задерживается между первым poll и входом в `lsp_completion`
- **GIVEN** completion request уже вошёл в service future scope, но ещё не достиг первой строки `lsp_completion`
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload содержит положительный `service_scope_to_method_wait_ms`
- **AND** оператор может отличить этот случай от lag до первого poll

### Requirement: `prepare_timeout` на `snapshot_with_deps` получает timeout-safe bounded runtime attribution (MUST)
Если `prepare_timeout` происходит после входа в фазу `snapshot_with_deps`, authoritative payload MUST уметь сериализовать bounded `snapshot_with_deps_timeout_runtime`, достаточный для различения overshoot как минимум между:
- `queue_wait`
- `exec`
- `wake_wait`
- `unavailable`

Object MUST оставаться bounded и MAY включать только:
- optional `queue_wait_ms`
- optional `exec_ms`
- optional `wake_wait_ms`
- required `resolution`

#### Scenario: Timeout происходит во время queue wait snapshot command
- **GIVEN** completion prepare timeout случается, пока `GetSnapshotWithDeps` ещё ждёт исполнения в runtime queue
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload включает `snapshot_with_deps_timeout_runtime`
- **AND** `resolution=queue_wait`
- **AND** payload не выдумывает `exec_ms`, если exec ещё не начался

#### Scenario: Timeout происходит после готового snapshot reply, но до timely wake
- **GIVEN** runtime уже завершил snapshot command, но completion future просыпается слишком поздно
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload включает bounded `snapshot_with_deps_timeout_runtime`
- **AND** `resolution=wake_wait`

#### Scenario: Timeout path ещё не имеет partial runtime split
- **GIVEN** prepare timeout произошёл на `snapshot_with_deps`, но bounded partial runtime attribution пока недоступна
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload использует `resolution=unavailable`
- **AND** не подменяет отсутствие данных guessed queue/exec/wake split
