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
- явно маркировать `Client Probe Feed` как local-only debug data, не эквивалентные server timeline.

Observability completion UI MUST NOT:
- реконструировать per-request server timeline из текстовых логов или агрегированных p50/p95/p99 метрик;
- использовать `TreeDataProvider` как реализацию timeline capability;
- подставлять отсутствующие server stages, routes или outcomes из client-side probe;
- скрывать server trace только потому, что local probes отсутствуют;
- выполнять trace-level correlation между server trace и local probes в рамках этого change.

#### Scenario: Пользователь видит самый тяжёлый этап completion-запроса
- **GIVEN** LSP вернул per-request completion trace со stage durations
- **WHEN** пользователь открывает server timeline в Observability UI
- **THEN** extension отображает этапы как визуальную временную шкалу с относительными длительностями
- **AND** этап с максимальной длительностью явно помечен как dominant/slow
- **AND** пользователю показаны `total_duration_ms` и `outcome` для конкретного server trace

#### Scenario: Панель корректно показывает отменённый completion
- **GIVEN** completion-запрос был отменён или superseded
- **WHEN** extension получает trace с terminal non-success outcome
- **THEN** `Server Timeline` отображает partial server timeline без фальшивого `completed` статуса
- **AND** terminal outcome отражается как cancelled/superseded для этого trace

#### Scenario: Timeline capability реализована только через webview и server-driven request
- **GIVEN** пользователь открыл completion observability UI
- **WHEN** extension обновляет server section панели
- **THEN** server trace запрашивается через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **AND** рендеринг выполняется в `webview` view, а не в `TreeDataProvider`

#### Scenario: Client Probe Feed показывается отдельно от Server Timeline
- **GIVEN** extension записала local client-side completion probes
- **WHEN** пользователь открывает completion observability UI
- **THEN** panel показывает отдельный `Client Probe Feed`
- **AND** local probes не встраиваются в server trace и не меняют `outcome` или `dominant_stage`

#### Scenario: Local probes не используются как суррогат server trace
- **GIVEN** extension записала local probes
- **WHEN** пользователь анализирует completion через Observability UI
- **THEN** panel не строит общую причинно-следственную timeline из local probes и server traces
- **AND** client-side данные остаются отдельным local-only debug stream

### Requirement: Timeline panel деградирует предсказуемо с legacy LSP (MUST)
Если подключённый LSP не поддерживает `bsl.getCompletionTimeline`, extension MUST fail-closed для server-side timeline capability:
- показывать явный user-facing статус несовместимости для `Server Timeline`;
- не падать и не ломать остальные разделы Observability/Sidebar;
- не маскировать отсутствие authoritative server timeline local probes-данными.

При этом `Client Probe Feed` MAY оставаться доступным как local-only debug stream, если probes уже записываются в extension.

#### Scenario: Legacy LSP не поддерживает server timeline request
- **GIVEN** `bsl.getCompletionTimeline` возвращает `Method not found`
- **WHEN** пользователь открывает completion observability UI
- **THEN** extension показывает понятное сообщение о неподдерживаемой версии сервера для `Server Timeline`
- **AND** оставляет рабочими другие observability views и команды
- **AND** если `Client Probe Feed` доступен, он явно помечен как local-only и не заменяет server timeline

## ADDED Requirements

### Requirement: VS Code extension ведёт bounded client-side completion probe buffer (MUST)
VS Code extension MUST вести bounded in-memory ring buffer последних client-side completion probes на основном activation/runtime path.

Probe buffer MUST:
- быть wired на default `LanguageClient` path, используемый обычной активацией extension;
- использовать deterministic oldest-first eviction;
- хранить только bounded/redacted probe fields;
- оставаться session-local и in-memory only для MVP.

Каждый probe MUST включать только bounded metadata:
- `probe_id`;
- `uri`;
- `document_version`;
- `trigger_mode` и optional `trigger_character`;
- `request_started_at_ms`;
- terminal status/result summary;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms` либо явное значение `unknown`, если этот сигнал недоступен;
- derived context flags вроде `is_after_dot` и `identifier_tail_length`.

Probe buffer MUST NOT:
- хранить raw document text, line prefixes или произвольные snippets;
- хранить unbounded free-form labels;
- требовать отдельного persistent telemetry pipeline в рамках этой capability.

#### Scenario: Default activation path записывает client-side completion probe
- **GIVEN** extension активирован через обычный `initializeLspClient` path
- **WHEN** пользователь вызывает completion в BSL документе
- **THEN** extension записывает probe в bounded in-memory buffer
- **AND** probe становится доступен для `Client Probe Feed`

#### Scenario: Переполнение probe buffer удаляет самый старый probe
- **GIVEN** probe buffer уже заполнен до configured max entries
- **WHEN** completion middleware записывает новый probe
- **THEN** удаляется самый старый probe
- **AND** новые probes остаются доступны в `Client Probe Feed`

#### Scenario: Probe payload остаётся redacted и bounded
- **GIVEN** completion вызван в документе с произвольным пользовательским кодом
- **WHEN** extension записывает client-side probe
- **THEN** probe содержит только bounded metadata и derived flags
- **AND** probe не содержит raw source text или unbounded snippets
