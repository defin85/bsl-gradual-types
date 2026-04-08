## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `24`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v21`, синхронизированный с текущим authoritative payload и его bounded field-set.

`v24` MUST сохранять additive `v23` ingress/query-body/flush-aware/output-egress semantics, включая grouped `query_bundle*` taxonomy, `response_sent_at_ms`, existing `response_output_*` milestones и `response_flush_completed_at_ms`.

Если `server_edge_details` присутствует, additive `v24` post-handler handoff split MAY включать:

- `response_output_handoff_started_at_ms`;
- `response_output_handoff_enqueued_at_ms`;
- `response_ready_to_output_handoff_wait_ms`;
- `response_output_handoff_send_wait_ms`;
- `response_output_handoff_to_writer_wait_ms`.

Если `response_output_handoff_started_at_ms` присутствует, payload MUST включать и `response_output_handoff_enqueued_at_ms`.

Если `response_output_handoff_started_at_ms` присутствует, payload MUST сохранять `response_output_enqueue_completed_at_ms` как legacy compatibility boundary output-writer selection для completion response и MUST включать все три derived fields:

- `response_ready_to_output_handoff_wait_ms`;
- `response_output_handoff_send_wait_ms`;
- `response_output_handoff_to_writer_wait_ms`.

Если `response_ready_to_output_handoff_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_sent_at_ms` и `response_output_handoff_started_at_ms` и MUST NOT включать blocking внутри outbound handoff path.

Если `response_output_handoff_send_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_output_handoff_started_at_ms` и `response_output_handoff_enqueued_at_ms` и MUST NOT включать wait после успешного handoff acceptance.

Если `response_output_handoff_to_writer_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_output_handoff_enqueued_at_ms` и `response_output_enqueue_completed_at_ms` и MUST NOT трактоваться как writer-queue backlog или конкретный blocker class без дополнительных authoritative fields.

Compatibility field `response_ready_to_output_enqueue_wait_ms` MAY сохраняться как umbrella интервал между `response_sent_at_ms` и `response_output_enqueue_completed_at_ms`, но MUST NOT переопределяться как точный синоним одного из новых `v24` buckets.

`response_output_enqueue_completed_at_ms` MUST NOT переосмысляться как truthful send-side enqueue completion для `v24`; это legacy compatibility field с writer-selection semantics, несмотря на историческое имя.

#### Scenario: Post-handler handoff gap отделён на три truthful bucket

- **GIVEN** completion handler уже подготовил response, outbound handoff начнётся позже, send-side acceptance завершится ещё позже, а output writer выберет completion response позже этого
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload сохраняет `response_sent_at_ms` и legacy `response_output_enqueue_completed_at_ms`
- **AND** публикует `response_output_handoff_started_at_ms`, `response_output_handoff_enqueued_at_ms`, `response_ready_to_output_handoff_wait_ms`, `response_output_handoff_send_wait_ms` и `response_output_handoff_to_writer_wait_ms` отдельно, если handoff boundaries наблюдаемы

#### Scenario: Legacy `response_output_enqueue_completed_at_ms` не выдаётся за truthful enqueue acceptance

- **GIVEN** authoritative payload содержит новый `v24` handoff split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_output_enqueue_completed_at_ms` трактуется как legacy writer-selection seam
- **AND** truthful send-side acceptance публикуется только через `response_output_handoff_enqueued_at_ms`

#### Scenario: Compatibility enqueue wait остаётся umbrella, а не переименованным bucket

- **GIVEN** authoritative payload содержит новый `v24` handoff split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_ready_to_output_enqueue_wait_ms` сохраняет compatibility semantics для полного интервала `response_sent_at_ms -> response_output_enqueue_completed_at_ms`
- **AND** consumer не трактует `v23` payload как будто truthful handoff boundaries уже были доступны

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v24`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v21` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v24/v21`, а не более старую версию
