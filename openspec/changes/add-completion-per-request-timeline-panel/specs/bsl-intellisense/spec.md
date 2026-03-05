## ADDED Requirements

### Requirement: VS Code extension показывает per-request completion timeline в панели Observability (MUST)
VS Code extension MUST предоставлять user-facing timeline view для последних completion-запросов в контейнере `bslAnalyzer`.

Timeline view MUST:
- читать данные только из server-driven LSP контракта `bsl.getCompletionTimeline`;
- показывать total duration, outcome и список stage entries для выбранного completion trace;
- визуально выделять dominant stage (самый длительный этап);
- отображать статус каждого этапа (`completed|cancelled|failed|skipped`).

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

### Requirement: Timeline panel деградирует предсказуемо с legacy LSP (MUST)
Если подключённый LSP не поддерживает `bsl.getCompletionTimeline`, extension MUST fail-closed для этой фичи:
- показывать явный user-facing статус несовместимости;
- не падать и не ломать остальные разделы Observability/Sidebar.

#### Scenario: Legacy LSP не поддерживает timeline request
- **GIVEN** `bsl.getCompletionTimeline` возвращает `Method not found`
- **WHEN** пользователь открывает timeline panel
- **THEN** extension показывает понятное сообщение о неподдерживаемой версии сервера
- **AND** оставляет рабочими другие observability views и команды

