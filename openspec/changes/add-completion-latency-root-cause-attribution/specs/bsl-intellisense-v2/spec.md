## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `6`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes, waiter states или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v6` MUST включать:
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
- `source` со значениями только из bounded vocabulary `prepare_guard|interactive_wait_budget`;
- `phase` из bounded prepare-phase vocabulary;
- `budget_ms`;
- `elapsed_ms`;
- `overshoot_ms`.

Runtime drilldown внутри `prepare_details` MUST использовать только bounded numeric/state fields и MUST NOT требовать свободного текста для интерпретации queue wait, wake path или snapshot execution.
Если timeout attribution присутствует без runtime reply details, payload MUST оставаться валидным и MUST NOT выдумывать отсутствующий runtime split.

Если `exact_wait` присутствует, объект MUST оставаться bounded и MUST включать существующие readiness/outcome поля, а также MAY включать bounded waiter/task-state поля и optional bounded `artifact_poll`, достаточные для различения как минимум:
- deadline на artifact polling path до waiter/task-state path;
- deadline уже после перехода в type-index waiter path;
- terminal readiness состояния head/exact artifact на последнем polling snapshot.

Если `artifact_poll` присутствует, объект MUST оставаться bounded и MAY включать только:
- `poll_count`;
- `poll_elapsed_ms`;
- `observed_file_version`;
- `head_ready`;
- `exact_ready`.

Если `server_edge_details` присутствует, объект MUST оставаться bounded и MUST включать:
- `transport_received_at_ms`;
- `handler_entered_at_ms`;
- `response_sent_at_ms`;
- optional `cancel_observed_at_ms`;
- `transport_to_handler_wait_ms`;
- `server_handler_exec_ms`;
- optional `cancel_observed_after_handler_enter_ms`;
- optional `method_entered_at_ms`;
- optional `transport_to_method_wait_ms`;
- optional `method_prelude_exec_ms`.

Если `method_entered_at_ms` присутствует, payload MUST включать и `transport_to_method_wait_ms`, и `method_prelude_exec_ms`, чтобы ingress attribution можно было прочитать без ручного вычитания timestamp'ов.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: Ingress-dominant trace различает queue/service lag и handler prelude
- **GIVEN** completion request пользователю ощущается как "долгий" ещё до основной completion-логики
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** authoritative payload содержит bounded данные, чтобы отделить `transport_received -> method_entered` от `method_entered -> handler_entered`
- **AND** existing `transport_to_handler_wait_ms` и `server_handler_exec_ms` остаются доступны для backward-compatible чтения

#### Scenario: Prepare timeout trace показывает timeout-layer и overshoot
- **GIVEN** completion завершается `prepare_timeout`
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded `timeout_attribution`
- **AND** payload явно показывает timeout-layer (`prepare_guard` или `interactive_wait_budget`)
- **AND** из `budget_ms`, `elapsed_ms` и `overshoot_ms` видно late timeout wake без чтения текстовых логов
- **AND** отсутствующие runtime reply details не выдумываются

#### Scenario: Exact deadline trace показывает artifact polling до waiter path
- **GIVEN** completion падает с `exact_deadline` до перехода в type-index waiter path
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded `exact_wait.artifact_poll`
- **AND** из него видно, сколько polling snapshot'ов было сделано и какие terminal readiness flags наблюдались
- **AND** payload не требует реконструкции artifact polling из косвенных global metrics

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v6` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

## ADDED Requirements

### Requirement: Existing completion surfaces переносят `v6` root-cause attribution без invented data (MUST)
VS Code extension MUST переносить authoritative `v6` root-cause attribution в уже существующие completion-oriented surface'ы, не требуя от оператора ручного чтения raw JSON для типовых verdict'ов.

Минимальные surface'ы:
- Completion Timeline panel;
- clipboard export видимого trace;
- observability incident handoff summary поверх authoritative timeline.

Derived projection MUST:
- строиться только из structured authoritative fields и bounded local status markers;
- различать `ingress_before_method_entry`, `handler_prelude_dominant`, `prepare_timeout@source` и `exact_deadline@artifact_poll`, когда соответствующие поля доступны;
- явно деградировать на payload `v5`;
- MUST NOT придумывать отсутствующие значения и MUST NOT подменять raw attachments.

#### Scenario: Completion Timeline panel показывает method-entry и timeout attribution
- **GIVEN** сервер вернул completion timeline с `v6` root-cause attribution
- **WHEN** пользователь открывает Completion Timeline panel
- **THEN** panel показывает bounded fact lines для method-entry split, timeout source/overshoot и artifact polling, если эти поля присутствуют
- **AND** оператору не требуется открывать raw JSON для типовых verdict'ов `handler_prelude_dominant`, `prepare_timeout@source` или `exact_deadline@artifact_poll`

#### Scenario: Clipboard export переносит ключевой `v6` verdict
- **GIVEN** пользователь копирует trace из Completion Timeline
- **WHEN** extension формирует clipboard text
- **THEN** copied text содержит ключевые bounded `v6` fact lines
- **AND** copied text не теряет distinction между transport wait, method prelude, timeout source и artifact polling, если эти поля присутствуют

#### Scenario: Incident handoff summary деградирует явно на payload `v5`
- **GIVEN** extension строит incident handoff summary для backend, который ещё не вернул `v6` root-cause attribution
- **WHEN** summary формируется из completion timeline payload версии `5`
- **THEN** summary остаётся валидным и использует доступные `v5` поля
- **AND** отсутствующие `v6` verdict details помечаются как unavailable, а не выдумываются
