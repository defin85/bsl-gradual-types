## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `9`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v6`, синхронизированный с текущим authoritative payload и его bounded field-set.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes, waiter states или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v9` MUST включать:
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
- `pre_method_attribution_provenance`;
- `handler_entered_at_ms`;
- `response_sent_at_ms`;
- optional `cancel_observed_at_ms`;
- `transport_to_handler_wait_ms`;
- `server_handler_exec_ms`;
- optional `cancel_observed_after_handler_enter_ms`;
- optional `method_entered_at_ms`;
- optional `transport_to_method_wait_ms`;
- optional `method_prelude_exec_ms`;
- optional `service_scope_entered_at_ms`;
- optional `transport_to_service_scope_wait_ms`;
- optional `service_scope_to_method_wait_ms`;
- optional `service_future_created_at_ms`;
- optional `transport_to_service_future_wait_ms`;
- optional `service_future_to_scope_wait_ms`.

Если `method_entered_at_ms` присутствует, payload MUST включать и `transport_to_method_wait_ms`, и `method_prelude_exec_ms`, чтобы ingress attribution можно было прочитать без ручного вычитания timestamp'ов.

Если `service_scope_entered_at_ms` присутствует, payload MUST включать и `transport_to_service_scope_wait_ms`, и `service_scope_to_method_wait_ms`, чтобы pre-method split оставался self-contained.

Если `service_future_created_at_ms` присутствует, payload MUST включать и `transport_to_service_future_wait_ms`, и `service_future_to_scope_wait_ms`, чтобы pre-service-scope split не требовал ручного вычитания timestamp'ов.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v9` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Ingress-dominant trace различает лаг до future, после future и handler prelude
- **GIVEN** completion request пользователю ощущается как "долгий" ещё до основной completion-логики
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** authoritative payload содержит bounded данные, чтобы отделить `transport_received -> service_future_created`, `service_future_created -> service_scope_entered` и `method_entered -> handler_entered`
- **AND** existing `transport_to_handler_wait_ms` и `server_handler_exec_ms` остаются доступны для backward-compatible чтения

#### Scenario: Prepare timeout trace показывает timeout-layer и overshoot
- **GIVEN** completion завершается `prepare_timeout`
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded `timeout_attribution`
- **AND** из `budget_ms`, `elapsed_ms` и `overshoot_ms` видно late timeout wake без чтения текстовых логов
- **AND** отсутствующие runtime reply details не выдумываются

#### Scenario: Exact deadline trace показывает artifact polling до waiter path
- **GIVEN** completion падает с `exact_deadline` до перехода в type-index waiter path
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded `exact_wait.artifact_poll`
- **AND** payload не требует реконструкции artifact polling из косвенных global metrics

#### Scenario: Versioned contract baseline синхронизирован с shipped payload
- **GIVEN** authoritative completion timeline уже публикует contract `v9`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** в `contracts/lsp-completion-timeline/v6/` существует новый contiguous baseline для текущего bounded payload
- **AND** older `v5` остаётся compatibility baseline для предыдущего `response.version=3` surface

## ADDED Requirements

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
