## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `23`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v20`, синхронизированный с текущим authoritative payload и его bounded field-set.

`v23` MUST сохранять additive `v22` ingress/query-body/flush-aware/output-egress semantics, включая grouped `query_bundle*` taxonomy, `response_sent_at_ms`, `response_output_*` milestones и `response_flush_completed_at_ms`.

Если `server_edge_details` присутствует, additive `v23` backlog attribution MAY включать:

- `output_messages_ahead_count`;
- `output_bytes_ahead_estimate`;
- `output_head_blocker_class`.

Если `output_messages_ahead_count` присутствует, это поле MUST описывать количество ahead outbound envelope для completion response на authoritative enqueue boundary и MUST включать active writer head, если writer уже занят.

Если `output_bytes_ahead_estimate` присутствует, это поле MUST описывать bounded best-effort estimate header+body bytes для ahead outbound envelope и MUST NOT требовать от consumer трактовки как exact flushed byte count.

Если `output_head_blocker_class` присутствует, это поле MUST использовать bounded vocabulary `completion|execute_command|other_request|notification|unknown` и MUST описывать coarse class ближайшего ahead blocker для completion response без raw payload fragments.

Backlog snapshot fields MUST NOT заменять existing `response_output_queue_wait_ms`; они должны оставаться supporting evidence для объяснения queue wait.

#### Scenario: Completion queue wait объясняется через truthful ahead snapshot

- **GIVEN** completion response успешно поставлен в unified outbound envelope path, пока writer уже занят или впереди стоят другие outbound envelopes
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload сохраняет existing `v22` egress split
- **AND** публикует bounded ahead snapshot, объясняющий `response_output_queue_wait_ms` без guessed reconstruction

#### Scenario: Snapshot semantics включает active writer head

- **GIVEN** writer уже пишет другой outbound envelope в момент enqueue completion response
- **WHEN** authoritative payload публикует `output_messages_ahead_count`
- **THEN** active writer head входит в ahead snapshot
- **AND** `output_head_blocker_class` может указывать на coarse class этого blocker

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v23`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v20` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v23/v20`, а не более старую версию
