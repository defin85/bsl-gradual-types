## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `2`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов или агрегированных observability-метрик.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
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

#### Scenario: Enriched local probes не меняют server timeline contract
- **GIVEN** VS Code extension записала local probes с дополнительными cancellation, transport, result-shape и overlap/drift diagnostics
- **WHEN** клиент вызывает `bsl.getCompletionTimeline`
- **THEN** response остаётся server-generated payload contract `v2`
- **AND** enriched local probe stream не меняет version или shape LSP timeline response

#### Scenario: Enriched local probes не подменяют server semantics
- **GIVEN** `Client Probe Feed` показывает extended local diagnostics
- **WHEN** пользователь анализирует completion observability
- **THEN** server trace остаётся authoritative representation LSP payload
- **AND** local diagnostics не подставляют server stages, routes, causes или outcomes
