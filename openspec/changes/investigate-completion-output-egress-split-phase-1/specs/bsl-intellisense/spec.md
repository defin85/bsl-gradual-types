## MODIFIED Requirements

### Requirement: Existing completion surfaces переносят `v22` output-egress split без guessed backlog attribution (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v22` finer output-egress split в человекочитаемом виде.

Human-readable projection MUST:

- показывать `response_ready_to_output_enqueue_wait_ms`, `response_output_queue_wait_ms`, `response_output_encode_exec_ms` и `response_output_write_and_flush_exec_ms`, если connected server возвращает `v22` payload с finer output-egress boundaries;
- MAY сохранять compatibility umbrella вроде `response_ready_to_flush_wait_ms`, но MUST NOT использовать её как единственный evidence bucket, если finer `v22` split доступен;
- сохранять existing `client_to_transport_wait_ms`, `transport_to_client_receive_wait_ms`, `client_receive_to_resolve_wait_ms` и `client_post_response_ms` как отдельные non-server buckets;
- явно деградировать на `v21`, не выдумывая enqueue/queue/encode/write buckets;
- не переименовывать `response_output_queue_wait_ms` в конкретный backlog culprit без отдельного authoritative backlog snapshot.

#### Scenario: Panel и clipboard показывают finer output-egress split

- **GIVEN** extension получает completion timeline `v22`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** output показывает enqueue wait, queue wait, encode exec и write/flush exec отдельно
- **AND** оператору не нужно читать raw JSON, чтобы увидеть finer server egress split

#### Scenario: Incident bundle summary не переименовывает coarse `v21` flush wait в точный culprit

- **GIVEN** connected server возвращает `v21` payload без finer output-egress split
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary явно отмечает, что finer output-egress split unavailable by design для этой evidence version
- **AND** derived handoff не называет coarse `response_ready_to_flush_wait_ms` точным queue backlog, encode bottleneck или write/flush bottleneck без новых bounded clocks
