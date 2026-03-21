## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `11`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v8`, синхронизированный с текущим authoritative payload и его bounded field-set.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes, waiter states или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v11` MUST включать:
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

Если `turn_attribution` присутствует, объект MUST оставаться bounded и MAY включать `dispatcher_resolution_latency_ms`, достаточный для отделения dispatcher-ready latency от остального ingress wait.

Если `prepare_details` присутствует, объект MUST оставаться bounded и MUST NOT вводить high-cardinality labels.
Для completion bottleneck drilldown этот объект MUST включать:
- `route` со значениями только из bounded vocabulary `head_hit|exact_hit` либо `null`;
- `fail_closed_cause` со значениями только из bounded vocabulary `prepare_timeout|exact_deadline` либо `null`;
- `progress` с coarse prepare phase и bounded offsets;
- optional bounded runtime drilldown для `wait_for_file_version`;
- optional bounded runtime drilldown для `snapshot_with_deps`;
- optional bounded `timeout_attribution`;
- optional `exact_wait`.

Если `timeout_attribution` присутствует, объект MUST оставаться bounded и MUST включать:
- `source`;
- `phase`;
- `budget_ms`;
- `elapsed_ms`;
- `overshoot_ms`.

Runtime drilldown внутри `prepare_details` MUST использовать только bounded numeric/state fields и MUST NOT требовать свободного текста для интерпретации queue wait, wake path или snapshot execution.
Если timeout attribution присутствует без runtime reply details, payload MUST оставаться валидным и MUST NOT выдумывать отсутствующий runtime split.

Если `exact_wait` присутствует, объект MUST оставаться bounded и MUST включать существующие readiness/outcome поля, а также MAY включать bounded waiter/task-state поля и optional bounded `artifact_poll`.

Если `artifact_poll` присутствует, объект MUST оставаться bounded и MAY включать только:
- `poll_count`;
- `poll_elapsed_ms`;
- `observed_file_version`;
- `head_ready`;
- `exact_ready`.

Если `server_edge_details` присутствует, объект MUST оставаться bounded и MUST включать:
- `transport_received_at_ms`;
- `transport_received_at_ms_provenance`;
- `pre_method_attribution_provenance`;
- `handler_entered_at_ms`;
- `response_sent_at_ms`;
- optional `cancel_observed_at_ms`;
- `transport_to_handler_wait_ms`;
- `server_handler_exec_ms`;
- optional `cancel_observed_after_handler_enter_ms`;
- optional `jsonrpc_dispatch_received_at_ms`;
- optional `dispatch_to_request_context_wait_ms`;
- optional `method_entered_at_ms`;
- optional `transport_to_method_wait_ms`;
- optional `method_prelude_exec_ms`;
- optional `service_scope_entered_at_ms`;
- optional `transport_to_service_scope_wait_ms`;
- optional `service_scope_to_method_wait_ms`;
- optional `service_future_created_at_ms`;
- optional `transport_to_service_future_wait_ms`;
- optional `service_future_to_scope_wait_ms`;
- optional `service_future_first_poll_entered_at_ms`;
- optional `service_future_to_first_poll_wait_ms`;
- optional `service_future_first_poll_outcome`;
- optional `service_future_first_wake_scheduled_at_ms`;
- optional `first_poll_to_first_wake_wait_ms`.

`transport_received_at_ms_provenance` MUST использовать только bounded vocabulary:
- `request_context_call_entry`;
- `jsonrpc_dispatch_received`.

`service_future_first_poll_outcome` MUST использовать только bounded vocabulary:
- `ready`;
- `pending`.

Если `method_entered_at_ms` присутствует, payload MUST включать и `transport_to_method_wait_ms`, и `method_prelude_exec_ms`, чтобы ingress attribution можно было прочитать без ручного вычитания timestamp'ов.

Если `service_scope_entered_at_ms` присутствует, payload MUST включать и `transport_to_service_scope_wait_ms`, и `service_scope_to_method_wait_ms`, чтобы pre-method split оставался self-contained.

Если `service_future_created_at_ms` присутствует, payload MUST включать и `transport_to_service_future_wait_ms`, и `service_future_to_scope_wait_ms`, чтобы pre-service-scope split не требовал ручного вычитания timestamp'ов.

Если `jsonrpc_dispatch_received_at_ms` присутствует, payload MUST включать и `dispatch_to_request_context_wait_ms`, чтобы pre-request-context split не требовал ручного вычитания timestamp'ов.

Если `service_future_first_poll_entered_at_ms` присутствует, payload MUST включать и `service_future_to_first_poll_wait_ms`, чтобы first-poll split не требовал ручного вычитания timestamp'ов.

Если `service_future_first_wake_scheduled_at_ms` присутствует, payload MUST включать и `first_poll_to_first_wake_wait_ms`, чтобы first-wake split не требовал ручного вычитания timestamp'ов.

Если `transport_received_at_ms_provenance=jsonrpc_dispatch_received`, payload MUST включать `jsonrpc_dispatch_received_at_ms`, а `transport_received_at_ms` MUST совпадать с ним.

Если `transport_received_at_ms_provenance=request_context_call_entry`, payload MUST NOT выдумывать `jsonrpc_dispatch_received_at_ms` и `dispatch_to_request_context_wait_ms`.

Если `service_future_first_poll_outcome=ready`, payload MUST NOT выдумывать `service_future_first_wake_scheduled_at_ms` и `first_poll_to_first_wake_wait_ms`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v11` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Ingress-dominant trace различает delay до first poll и delay после первого `Pending`
- **GIVEN** completion request пользователю ощущается как "долгий" ещё до основной completion-логики
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** authoritative payload содержит bounded данные, чтобы отделить `service_future_created -> first_poll` от `first_poll(Pending) -> first_wake`
- **AND** existing `service_future_to_scope_wait_ms`, `transport_to_handler_wait_ms` и `server_handler_exec_ms` остаются доступны для backward-compatible чтения

#### Scenario: Первый poll сразу `Ready`
- **GIVEN** returned service future завершает первый poll без pending path
- **WHEN** сервер сериализует completion timeline `v11`
- **THEN** payload включает bounded first-poll timestamp и outcome `ready`
- **AND** payload не выдумывает first-wake fields

#### Scenario: Versioned contract baseline синхронизирован с shipped payload
- **GIVEN** authoritative completion timeline уже публикует contract `v11`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** в `contracts/lsp-completion-timeline/v8/` существует новый contiguous baseline для текущего bounded payload
- **AND** older `v7` остаётся compatibility baseline для предыдущего `response.version=10` surface

## ADDED Requirements

### Requirement: `v11` service-future poll / wake split сохраняет truthful post-dispatch attribution semantics (MUST)
Новый bounded split внутри `service_future_created -> service_scope_entered` MUST не ослаблять existing `v10` / `v9` / `v8` integrity semantics.

Сервер MUST:
- сохранять existing `transport_received_at_ms_provenance` и `pre_method_attribution_provenance`;
- не подменять отсутствие first-poll или first-wake observation guessed полями;
- не добавлять free-text/high-cardinality debug fields;
- явно сообщать bounded outcome первого poll через `service_future_first_poll_outcome`, если first poll наблюдался.

#### Scenario: Первый poll вернул `Pending`, но первый wake не наблюдался
- **GIVEN** completion timeline trace знает момент первого poll и знает, что он вернул `Pending`
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload честно включает `service_future_first_poll_outcome=pending`
- **AND** payload не выдумывает `service_future_first_wake_scheduled_at_ms`, если first wake не наблюдался
- **AND** payload не выдумывает `first_poll_to_first_wake_wait_ms`

#### Scenario: Connected server ещё не поддерживает `v11`
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension или operator читает authoritative payload
- **THEN** payload не выдумывает first-poll / first-wake split
- **AND** trustworthy semantics остаются ограничены уже существующими `v10` полями
