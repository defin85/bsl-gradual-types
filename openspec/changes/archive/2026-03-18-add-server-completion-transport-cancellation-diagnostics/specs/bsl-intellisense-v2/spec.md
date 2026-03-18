## MODIFIED Requirements
### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `3`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов или агрегированных observability-метрик.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v3` MUST включать:
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

Если `prepare_details` присутствует, объект MUST оставаться bounded и MUST NOT вводить high-cardinality labels.
Для split-prepare routing этот объект MUST содержать:
- `route` со значениями только из bounded vocabulary `head_hit|exact_hit` либо `null`;
- `fail_closed_cause` со значениями только из bounded vocabulary `prepare_timeout|exact_deadline` либо `null`.

Если `server_edge_details` присутствует, объект MUST оставаться bounded и MUST включать:
- `transport_received_at_ms`;
- `handler_entered_at_ms`;
- `response_sent_at_ms`;
- optional `cancel_observed_at_ms`;
- `transport_to_handler_wait_ms`;
- `server_handler_exec_ms`;
- optional `cancel_observed_after_handler_enter_ms`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: Long empty completion trace несёт authoritative server-edge breakdown
- **GIVEN** completion request завершился без items и без client-side correlation данных
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** server trace MAY содержать bounded `server_edge_details`
- **AND** этих данных достаточно, чтобы отделить `transport_to_handler_wait` от `server_handler_exec`
- **AND** existing `prepare_details` и `stages` остаются частью authoritative payload

#### Scenario: Late cancellation trace фиксирует момент backend observation
- **GIVEN** completion request был отменён уже после старта server handler
- **WHEN** LSP формирует authoritative timeline trace
- **THEN** trace MAY содержать `cancel_observed_at_ms`
- **AND** trace MAY содержать `cancel_observed_after_handler_enter_ms`
- **AND** payload не выдумывает client-side obsolete timestamp

#### Scenario: Enriched local probes не меняют server timeline contract
- **GIVEN** VS Code extension записала local probes с дополнительными cancellation, transport, result-shape и overlap/drift diagnostics
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response остаётся server-generated payload contract `v3`
- **AND** enriched local probe stream не меняет version или shape LSP timeline response

## ADDED Requirements
### Requirement: Completion transport/cancellation observability остаётся bounded и completion-specific (MUST)
Server-side observability для transport/cancellation diagnostics MUST оставаться bounded и completion-specific.

Instrumentation MUST:
- записывать bounded latency samples для `transport_to_handler_wait`;
- записывать bounded latency samples для `server_handler_exec`;
- записывать bounded cancellation observability только для completion path;
- не вводить high-cardinality metric labels или free-form cancellation reasons.

#### Scenario: Cancellation observability не взрывает cardinality
- **GIVEN** completion requests отменяются для разных документов и запросов
- **WHEN** сервер пишет transport/cancellation observability
- **THEN** новые metric keys остаются в fixed low-cardinality vocabulary
- **AND** URI, snippets и произвольные reason strings не попадают в metric labels
