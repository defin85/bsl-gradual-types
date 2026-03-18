## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `2`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов или агрегированных observability-метрик.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, но такой local-only debug stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v2` MUST включать:
- `version` (числовой номер контракта);
- `traces` (массив completion trace записей).

Каждый trace MUST включать:
- `trace_id`, `request_id`, `uri`, `trigger_mode`;
- `outcome`, `started_at_ms`, `total_duration_ms`;
- `dominant_stage`;
- `prepare_details`;
- `turn_attribution`;
- `stages`.

Если `prepare_details` присутствует, объект MUST оставаться bounded и MUST NOT вводить high-cardinality labels.
Для split-prepare routing этот объект MUST содержать:
- `route` со значениями только из bounded vocabulary `head_hit|exact_hit` либо `null`;
- `fail_closed_cause` со значениями только из bounded vocabulary `prepare_timeout|exact_deadline` либо `null`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: Клиент получает детерминированный timeline для завершённого completion
- **GIVEN** completion-запрос успешно обработан
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response содержит trace со стадиями в порядке исполнения
- **AND** `total_duration_ms` не меньше максимального stage end offset
- **AND** `dominant_stage` совпадает с этапом максимальной длительности в trace

#### Scenario: Клиент получает корректный timeline для cancelled/superseded completion
- **GIVEN** completion-запрос отменён или superseded до полного завершения pipeline
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response содержит partial trace с terminal outcome cancelled/superseded
- **AND** trace не маркируется как успешный completed

#### Scenario: VS Code клиент получает timeline через `workspace/executeCommand`
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v2` с server-generated traces
- **AND** клиент не строит server timeline из текстовых логов или p95/p99 агрегатов

#### Scenario: Local probe stream не меняет server trace semantics
- **GIVEN** VS Code extension показывает рядом server timeline и local client-side probes
- **WHEN** UI показывает completion observability details
- **THEN** server trace остаётся неизменным representation LSP payload
- **AND** client-side probe не подставляет отсутствующие server stages, routes или outcomes

#### Scenario: Local probe stream не становится частью LSP timeline contract
- **GIVEN** extension записала local client-side probes
- **WHEN** пользователь открывает completion observability UI
- **THEN** `bsl.getCompletionTimeline` продолжает возвращать только server-generated traces
- **AND** local probe stream не меняет version или shape LSP timeline response

#### Scenario: Timeline раскрывает bounded split-prepare routing без cardinality drift
- **GIVEN** completion обслужен через current-revision `head` path или fail-closed exact wait
- **WHEN** клиент читает `prepare_details` в trace
- **THEN** `route` остаётся только в bounded vocabulary `head_hit|exact_hit` или `null`
- **AND** `fail_closed_cause` остаётся только в bounded vocabulary `prepare_timeout|exact_deadline` или `null`
- **AND** timeline не включает динамические route/cause labels
