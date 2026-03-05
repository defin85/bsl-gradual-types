## ADDED Requirements

### Requirement: VS Code extension показывает per-request completion timeline в панели Observability (MUST)
VS Code extension MUST предоставлять user-facing timeline view для последних completion-запросов в контейнере `bslAnalyzer`.

Timeline view MUST:
- читать данные только из server-driven LSP контракта `bsl.getCompletionTimeline`;
- читать данные через request path `workspace/executeCommand` (`command: bsl.getCompletionTimeline`) как единственный transport для этой capability;
- быть реализован как `webview` view (`WebviewViewProvider`) внутри контейнера `bslAnalyzer`;
- показывать total duration, outcome и список stage entries для выбранного completion trace;
- визуально выделять dominant stage (самый длительный этап);
- отображать статус каждого этапа (`completed|cancelled|failed|skipped`).

Timeline view MUST NOT:
- реконструировать per-request timeline из текстовых логов или агрегированных p50/p95/p99 метрик;
- использовать `TreeDataProvider` как реализацию timeline capability.

#### Scenario: Пользователь видит самый тяжёлый этап completion-запроса
- **GIVEN** LSP вернул per-request completion trace со stage durations
- **WHEN** пользователь открывает timeline panel
- **THEN** extension отображает этапы как визуальную временную шкалу с относительными длительностями
- **AND** этап с максимальной длительностью явно помечен как dominant/slow
- **AND** пользователю показаны `total_duration_ms` и `outcome` для конкретного trace

#### Scenario: Панель корректно показывает отменённый completion
- **GIVEN** completion-запрос был отменён или superseded
- **WHEN** extension получает trace с terminal non-success outcome
- **THEN** panel отображает partial timeline без фальшивого "completed" статуса
- **AND** terminal outcome отражается как cancelled/superseded для этого trace

#### Scenario: Timeline capability реализована только через webview и server-driven request
- **GIVEN** пользователь открыл раздел completion timeline в Observability
- **WHEN** extension обновляет данные панели
- **THEN** данные запрашиваются через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **AND** рендеринг выполняется в `webview` view, а не в `TreeDataProvider`

### Requirement: Timeline panel деградирует предсказуемо с legacy LSP (MUST)
Если подключённый LSP не поддерживает `bsl.getCompletionTimeline`, extension MUST fail-closed для этой фичи:
- показывать явный user-facing статус несовместимости;
- не падать и не ломать остальные разделы Observability/Sidebar.

#### Scenario: Legacy LSP не поддерживает timeline request
- **GIVEN** `bsl.getCompletionTimeline` возвращает `Method not found`
- **WHEN** пользователь открывает timeline panel
- **THEN** extension показывает понятное сообщение о неподдерживаемой версии сервера
- **AND** оставляет рабочими другие observability views и команды
