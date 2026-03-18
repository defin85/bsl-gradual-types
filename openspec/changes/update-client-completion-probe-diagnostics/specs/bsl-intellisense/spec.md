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
- отображать в `Client Probe Feed`, когда они доступны, bounded cancellation diagnostics, transport-phase diagnostics, result-shape diagnostics и version-drift/overlap diagnostics.

Observability completion UI MUST NOT:
- реконструировать per-request server timeline из текстовых логов или агрегированных p50/p95/p99 метрик;
- использовать `TreeDataProvider` как реализацию timeline capability;
- подставлять отсутствующие server stages, routes или outcomes из client-side probe;
- скрывать server trace только потому, что local probes отсутствуют;
- выполнять trace-level correlation между server trace и local probes в рамках этого change.

#### Scenario: Пользователь отличает superseded cancel от пустого completion
- **GIVEN** `Client Probe Feed` содержит отменённый probe и bounded cancellation diagnostics
- **WHEN** пользователь открывает completion observability UI
- **THEN** panel показывает `client_terminal_state=cancelled`
- **AND** если доступен `cancel_reason_hint`, он отображается как local diagnostic hint
- **AND** `Client Probe Feed` не подменяет этим hint server `outcome`

#### Scenario: Пользователь видит transport-phase breakdown для длинного local probe
- **GIVEN** `Client Probe Feed` содержит explicit transport-phase timestamps для completion probe
- **WHEN** пользователь открывает completion observability UI
- **THEN** panel показывает enough-local diagnostics, чтобы отличить pre-send delay, LSP/in-flight wait и post-response overhead
- **AND** эти client-side diagnostics остаются отдельными от `Server Timeline`

#### Scenario: Пользователь видит version drift и overlap context без correlation guesswork
- **GIVEN** completion probe пережил локальные правки, движение курсора или overlap с новыми completion probes
- **WHEN** пользователь открывает completion observability UI
- **THEN** `Client Probe Feed` показывает bounded version-drift/overlap diagnostics
- **AND** UI не строит machine-join между этим probe и конкретным server trace

### Requirement: VS Code extension ведёт bounded client-side completion probe buffer (MUST)
VS Code extension MUST вести bounded in-memory ring buffer последних client-side completion probes на основном activation/runtime path.

Probe buffer MUST:
- быть wired на default `LanguageClient` path, используемый обычной активацией extension;
- использовать deterministic oldest-first eviction;
- хранить только bounded/redacted probe fields;
- оставаться session-local и in-memory only.

Каждый probe MUST включать только bounded metadata:
- `probe_id`;
- `uri`;
- `document_version`;
- `document_version_at_terminal`;
- `trigger_mode` и optional `trigger_character`;
- `request_started_at_ms`;
- `request_completed_at_ms`;
- explicit transport-phase milestones, достаточные для отделения client enter, LSP dispatch, LSP response receive и client terminal;
- terminal status/result summary;
- bounded `result_kind` vocabulary;
- bounded `item_count_bucket`;
- `is_incomplete`, только если этот сигнал доступен без guesswork;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms` либо явное значение `unknown`, если этот сигнал недоступен;
- bounded cancellation diagnostics: `cancel_reason_hint` из vocabulary `superseded_same_version|superseded_newer_version|editor_state_changed|unknown`, optional `superseded_by_probe_id`, optional `superseded_after_ms`;
- bounded overlap/drift diagnostics: `did_change_count_during_probe`, `cursor_moved_during_probe`, `active_completion_count_at_start`, `same_uri_probe_overlap_count`, `newer_probe_started_before_terminal`;
- derived context flags вроде `is_after_dot` и `identifier_tail_length`.

Probe buffer MUST NOT:
- хранить raw document text, line prefixes или произвольные snippets;
- хранить unbounded free-form labels;
- требовать отдельного persistent telemetry pipeline в рамках этой capability;
- требовать protocol-level `client_probe_id` или trace-level correlation с `Server Timeline`.

#### Scenario: Superseded completion probe получает bounded cancellation diagnostics
- **GIVEN** completion probe был отменён после старта более нового completion probe на том же `uri`
- **WHEN** extension завершает запись client-side probe
- **THEN** probe содержит `client_terminal_state=cancelled`
- **AND** probe содержит bounded `cancel_reason_hint`
- **AND** если superseding probe известен локально, probe MAY содержать `superseded_by_probe_id` и `superseded_after_ms`

#### Scenario: Успешный пустой completion probe содержит result-shape и transport diagnostics
- **GIVEN** completion probe завершился без cancellation и без items
- **WHEN** extension записывает probe
- **THEN** probe содержит bounded `result_kind` и `item_count_bucket`
- **AND** probe содержит transport-phase milestones для локального разбора длительности
- **AND** probe не требует реконструкции server timeline

#### Scenario: Probe фиксирует drift и overlap контекст во время жизни запроса
- **GIVEN** во время completion probe произошли дополнительные правки, движение курсора или запуск новых completion probes на том же документе
- **WHEN** extension завершает запись probe
- **THEN** probe содержит bounded drift/overlap diagnostics
- **AND** probe остаётся redacted и session-local
