## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `5`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes, waiter states или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v5` MUST включать:
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

Если `turn_attribution` присутствует, объект MUST оставаться bounded и MAY включать `dispatcher_resolution_latency_ms`, достаточный для отделения dispatcher-ready latency от остального `transport_to_handler_wait`.

Если `prepare_details` присутствует, объект MUST оставаться bounded и MUST NOT вводить high-cardinality labels.
Для completion bottleneck drilldown этот объект MUST включать:
- `route` со значениями только из bounded vocabulary `head_hit|exact_hit` либо `null`;
- `fail_closed_cause` со значениями только из bounded vocabulary `prepare_timeout|exact_deadline` либо `null`;
- `progress` с coarse prepare phase и bounded offsets;
- optional bounded runtime drilldown для `wait_for_file_version`;
- optional bounded runtime drilldown для `snapshot_with_deps`;
- optional `exact_wait`.

Runtime drilldown внутри `prepare_details` MUST использовать только bounded numeric/state fields и MUST NOT требовать свободного текста для интерпретации queue wait, wake path или snapshot execution.

Если `exact_wait` присутствует, объект MUST оставаться bounded и MUST включать существующие readiness/outcome поля, а также MAY включать bounded waiter/task-state поля, достаточные для различения как минимум:
- matching task присутствует или отсутствует;
- waiter только joined существующий task или promoted background task;
- task находится в одной из bounded phase категорий ожидания/вычисления.

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

#### Scenario: Ingress-dominant trace различает transport, dispatcher и handler work
- **GIVEN** completion request пользователю ощущается как "долгий" до входа в основную логику completion
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** authoritative payload содержит достаточно bounded данных, чтобы отделить `transport_to_handler_wait` от `server_handler_exec`
- **AND** при наличии `dispatcher_resolution_latency_ms` видно, была ли задержка уже после dispatcher-ready

#### Scenario: Prepare timeout trace показывает subphase и runtime split
- **GIVEN** completion завершается `prepare_timeout`
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded prepare phase marker
- **AND** payload явно показывает, относится ли bottleneck к `wait_for_file_version` или `snapshot_with_deps`
- **AND** при наличии runtime drilldown можно отличить queue wait от execute/wake path без чтения текстовых логов

#### Scenario: Exact deadline trace показывает waiter/task state
- **GIVEN** completion падает с `exact_deadline` после успешного `prepare`
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** trace содержит bounded `exact_wait` block
- **AND** из него видно outcome exact wait, waiter action и task-state категорию
- **AND** payload не требует реконструкции exact root cause из косвенных global metrics

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v5` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

## ADDED Requirements

### Requirement: Человекочитаемые completion timeline projections сохраняют authoritative bottleneck semantics (MUST)
VS Code extension MUST проецировать bounded authoritative bottleneck drilldown из completion timeline в человекочитаемые surface'ы, не заставляя оператора читать raw JSON для типовых root-cause verdict'ов.

Минимальные surface'ы:
- Completion Timeline panel;
- clipboard export видимого trace;
- AI-friendly incident handoff summary поверх authoritative timeline.

Derived projection MUST:
- строиться только из structured authoritative fields и bounded local status markers;
- явно различать `ingress_dominant`, `prepare_timeout` subphase и `exact_wait` bottleneck, когда соответствующие поля доступны;
- деградировать явно, если backend вернул старый payload или часть новых bounded полей отсутствует;
- MUST NOT придумывать отсутствующие значения и MUST NOT подменять raw attachments.

#### Scenario: Completion Timeline panel показывает bounded bottleneck drilldown
- **GIVEN** сервер вернул completion timeline с bounded prepare/exact/dispatcher drilldown
- **WHEN** пользователь открывает Completion Timeline panel
- **THEN** panel показывает эти факты в человекочитаемом виде рядом с trace
- **AND** оператору не требуется открывать raw JSON для типового verdict `ingress_dominant`, `prepare_timeout@phase` или `exact_deadline`

#### Scenario: Clipboard export переносит ключевой bottleneck verdict
- **GIVEN** пользователь копирует trace из Completion Timeline
- **WHEN** extension формирует clipboard text
- **THEN** copied text содержит ключевые bounded drilldown поля
- **AND** copied text не теряет distinction между `transport_to_handler_wait`, `dispatcher_resolution_latency_ms`, `prepare` subphase и `exact_wait` state, если эти поля присутствуют

#### Scenario: Incident handoff summary деградирует явно на payload `v4`
- **GIVEN** extension строит incident handoff summary для backend, который ещё не вернул `v5` drilldown
- **WHEN** summary формируется из completion timeline payload более старой версии
- **THEN** summary остаётся валидным и использует доступные `v4` поля
- **AND** отсутствующие `v5` verdict details помечаются как unavailable, а не выдумываются
