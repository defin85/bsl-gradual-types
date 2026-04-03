## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `22`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v19`, синхронизированный с текущим authoritative payload и его bounded field-set.

`v22` MUST сохранять additive `v21` ingress/query-body/flush-aware semantics, включая grouped `query_bundle*` taxonomy, `response_sent_at_ms` response-ready semantics и `response_flush_completed_at_ms`.

Контракт `v22` MUST включать:

- `version` (числовой номер контракта);
- `traces` (массив completion trace записей).

Каждый trace MUST включать:

- `trace_id`, `request_id`, `uri`, `trigger_mode`;
- `outcome`, `started_at_ms`, `total_duration_ms`;
- `dominant_stage`;
- `prepare_details`;
- `turn_attribution`;
- optional `server_edge_details`;
- `stages`.

Если `server_edge_details` присутствует, additive `v22` output-egress split MUST сохранять existing semantics `response_sent_at_ms` и MAY включать:

- `response_output_enqueue_completed_at_ms`;
- `response_output_write_started_at_ms`;
- `response_output_encode_completed_at_ms`;
- `response_flush_completed_at_ms`;
- `response_ready_to_output_enqueue_wait_ms`;
- `response_output_queue_wait_ms`;
- `response_output_encode_exec_ms`;
- `response_output_write_and_flush_exec_ms`.

`response_sent_at_ms` MUST продолжать обозначать handler-local response-ready boundary. Это поле MUST NOT ретроактивно переосмысляться как output enqueue completion, write start или flush completion.

Если `response_output_enqueue_completed_at_ms` присутствует, payload MUST включать и `response_ready_to_output_enqueue_wait_ms`, чтобы enqueue wait не требовал ручного вычитания timestamp'ов.

Если `response_output_write_started_at_ms` присутствует, payload MUST включать и `response_output_queue_wait_ms`, чтобы ожидание в output path не требовало ручного вычитания timestamp'ов.

Если `response_output_encode_completed_at_ms` присутствует, payload MUST включать и `response_output_encode_exec_ms`, и `response_output_write_and_flush_exec_ms`, чтобы encode и write/flush не оставались смешанными в одном bucket.

Если `response_output_queue_wait_ms`, `response_output_encode_exec_ms` или `response_output_write_and_flush_exec_ms` присутствуют, эти поля MUST описывать только server-side output-egress intervals и MUST NOT включать client-side transport, promise resolution или extension-host post-receive wait.

#### Scenario: VS Code клиент получает `v22` payload без reconstruction

- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v22` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Output egress split виден сразу после completion response

- **GIVEN** completion handler уже подготовил response, а output path зафиксировал enqueue/write/encode/flush milestones
- **WHEN** immediate follow-up читает `bsl.getCompletionTimeline`
- **THEN** payload сохраняет `response_sent_at_ms` как handler-ready boundary
- **AND** публикует целостный `v22` egress split без partial patch state

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v22`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v19` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v22/v19`, а не более старую версию

#### Scenario: Legacy `response_sent_at_ms` и `response_flush_completed_at_ms` semantics не меняется задним числом

- **GIVEN** authoritative payload содержит новый output-egress split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_sent_at_ms` сохраняет response-ready semantics
- **AND** `response_flush_completed_at_ms` продолжает обозначать flush completion
- **AND** finer enqueue/write/encode boundaries публикуются только через additive `v22` поля
