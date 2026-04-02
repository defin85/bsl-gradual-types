## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `20`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v17`, синхронизированный с текущим authoritative payload и его bounded field-set.

Контракт `v20` MUST включать:
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

Если `server_edge_details` присутствует, additive `v19` ingress split (`adapter_read_at_ms`, `adapter_to_dispatch_wait_ms`) MUST сохраняться без переосмысления legacy `transport_received_at_ms`.

Timeline stage taxonomy в `v20` MUST оставаться bounded и MAY включать detailed `query_bundle` stage names:
- `query_bundle_pool_wait`
- `query_bundle_deps_and_file_snapshot`
- `query_bundle_ir_query`
- `query_bundle_ir_retry`
- `query_bundle_other`

Если trace использует detailed `query_bundle` stage names, `dominant_stage` MAY ссылаться на конкретный `query_bundle*` stage вместо legacy aggregate `query_bundle`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v20` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Versioned contract baseline синхронизирован с shipped payload
- **GIVEN** authoritative completion timeline уже публикует contract `v20`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v17` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v20/v17`, а не более старую версию

#### Scenario: Query-body stage taxonomy остаётся bounded
- **GIVEN** completion trace использует detailed `query_bundle` attribution
- **WHEN** сервер сериализует timeline payload
- **THEN** trace использует только stage names из bounded `query_bundle*` vocabulary
- **AND** payload не вводит high-cardinality stage labels для отдельных файлов, URI или запросов

## ADDED Requirements

### Requirement: Query-body timeline attribution остаётся truthful на success, cancel и fail paths (MUST)
Если request вошёл в query-body path completion, authoritative timeline MUST публиковать query-body spent time как stage accounting, а не терять его целиком в `unattributed_overhead`.

Для `v20` это означает:
- request-local trace MUST публиковать хотя бы один `query_bundle*` stage после входа в query-body path;
- stage status MUST отражать terminal path (`completed`, `cancelled` или `failed`);
- bounded request-local attribution SHOULD отделять pool wait от blocking query execution;
- trace MUST NOT выдумывать hard preemption или cancellation checkpoint внутри compute, если его реально не было.

#### Scenario: Cancelled request внутри query-body сохраняет stage accounting
- **GIVEN** completion request вошёл в query-body path и затем был superseded или cancelled
- **WHEN** authoritative timeline сериализуется после terminal outcome
- **THEN** trace содержит `query_bundle*` stage со статусом `cancelled`
- **AND** seconds-scale handler tail не исчезает целиком в `unattributed_overhead`

#### Scenario: Pool saturation отделена от actual blocking compute
- **GIVEN** interactive completion ждёт permit в bounded blocking runtime, а затем выполняет тяжёлый IR query
- **WHEN** authoritative timeline сериализует query-body attribution
- **THEN** trace позволяет отличить `query_bundle_pool_wait` от `query_bundle_ir_query`
- **AND** incident analysis не обязан восстанавливать этот split только по глобальным метрикам

#### Scenario: Failed query-body path не публикуется как успешный aggregate
- **GIVEN** query-body path завершился ошибкой до normal response build
- **WHEN** authoritative timeline формируется для этого request
- **THEN** trace публикует `query_bundle*` stage со статусом `failed`
- **AND** stage accounting остаётся truthful даже без successful completion result
