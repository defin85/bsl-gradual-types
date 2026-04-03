## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `21`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v18`, синхронизированный с текущим authoritative payload и его bounded field-set.

`v21` MUST сохранять additive `v20` ingress/query-body semantics, включая grouped `query_bundle*` taxonomy и existing bounded server-edge fields.

Контракт `v21` MUST включать:

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

Если `server_edge_details` присутствует, additive `v21` post-handler egress split MUST сохранять existing semantics `response_sent_at_ms` и MAY включать:

- `response_flush_completed_at_ms`;
- `response_ready_to_flush_wait_ms`.

`response_sent_at_ms` MUST продолжать обозначать handler-local response-ready boundary. Это поле MUST NOT ретроактивно переосмысляться как transport flush completion.

Если `response_flush_completed_at_ms` присутствует, payload MUST включать и `response_ready_to_flush_wait_ms`, чтобы post-handler server egress split не требовал ручного вычитания timestamp'ов.

Если `response_ready_to_flush_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_sent_at_ms` и фактическим flush completion для этого response и MUST NOT включать client-side transport или extension-host post-receive wait.

#### Scenario: VS Code клиент получает `v21` payload без reconstruction

- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v21` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Flush completion отделён от handler-ready boundary

- **GIVEN** completion handler уже подготовил response, но transport flush завершится позже
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload сохраняет `response_sent_at_ms` как handler-ready boundary
- **AND** публикует `response_flush_completed_at_ms` и `response_ready_to_flush_wait_ms` отдельно, если flush boundary наблюдаема

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v21`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v18` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v21/v18`, а не более старую версию

#### Scenario: Legacy `response_sent_at_ms` semantics не меняется задним числом

- **GIVEN** authoritative payload содержит новый flush-aware split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_sent_at_ms` сохраняет response-ready semantics
- **AND** transport flush completion публикуется только через additive `response_flush_completed_at_ms`
