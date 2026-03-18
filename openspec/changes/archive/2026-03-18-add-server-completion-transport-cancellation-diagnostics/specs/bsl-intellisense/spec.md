## MODIFIED Requirements
### Requirement: VS Code extension показывает per-request completion timeline в панели Observability (MUST)
VS Code extension MUST предоставлять user-facing completion observability UI в контейнере `bslAnalyzer`.

Observability completion UI MUST:
- читать authoritative server trace только из server-driven LSP контракта `bsl.getCompletionTimeline`;
- читать server trace через request path `workspace/executeCommand` (`command: bsl.getCompletionTimeline`) как единственный transport для server-side части этой capability;
- быть реализован как `webview` view (`WebviewViewProvider`) внутри контейнера `bslAnalyzer`;
- содержать отдельный `Server Timeline` section;
- содержать отдельный local-only `Client Probe Feed` section;
- показывать total duration, outcome и список stage entries для выбранного server trace;
- визуально выделять dominant stage (самый длительный server этап);
- отображать статус каждого server этапа (`completed|cancelled|failed|skipped`);
- явно маркировать `Client Probe Feed` как local-only debug data, не эквивалентные server timeline;
- отображать в `Client Probe Feed`, когда они доступны, bounded cancellation diagnostics, transport-phase diagnostics, result-shape diagnostics и version-drift/overlap diagnostics;
- отображать в `Server Timeline`, когда они доступны, bounded server-edge transport/cancellation diagnostics из authoritative server trace.

Observability completion UI MUST NOT:
- реконструировать per-request server timeline из текстовых логов или агрегированных p50/p95/p99 метрик;
- использовать `TreeDataProvider` как реализацию timeline capability;
- подставлять отсутствующие server stages, routes или outcomes из client-side probe;
- скрывать server trace только потому, что local probes отсутствуют;
- выполнять trace-level correlation между server trace и local probes в рамках этого change;
- подставлять server-edge diagnostics из client-side probe, если серверный payload их не содержит.

#### Scenario: Пользователь отличает queue-before-handler от долгого server execution
- **GIVEN** authoritative `Server Timeline` trace содержит `server_edge_details`
- **WHEN** пользователь открывает completion observability UI
- **THEN** panel показывает bounded server-edge diagnostics для `transport_to_handler_wait` и `server_handler_exec`
- **AND** эти diagnostics остаются частью `Server Timeline`, а не `Client Probe Feed`

#### Scenario: Legacy timeline payload без server-edge diagnostics остаётся читаемым
- **GIVEN** connected server возвращает payload `version=2` без `server_edge_details`
- **WHEN** пользователь открывает completion observability UI
- **THEN** extension продолжает показывать `Server Timeline` и `Client Probe Feed`
- **AND** не пытается выдумывать отсутствующие server-edge diagnostics
- **AND** отсутствие новых полей не ломает rendering/copy flow
