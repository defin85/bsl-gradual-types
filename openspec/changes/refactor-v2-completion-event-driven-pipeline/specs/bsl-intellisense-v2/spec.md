## ADDED Requirements

### Requirement: Completion v2 использует per-file event-driven orchestrator для интерактивного пути (MUST)
Система MUST обрабатывать интерактивный completion через per-file event-driven orchestrator (`dispatcher/actor`) с явными событиями `DidOpen`, `DidChange`, `CompletionRequest`, `Cancel`, `DidClose`.

Очередь событий per-file orchestrator MUST быть bounded, а policy переполнения MUST сохранять latest-wins семантику для интерактивного completion (устаревшие запросы коалесцируются/вытесняются, а не копятся без ограничений).

`didChange` ingest MUST оставаться неблокирующим относительно интерактивного completion пути.

Система MUST ограничивать интерактивный tail latency под warm-нагрузкой с измеримыми SLO-гейтами rollout.

#### Scenario: Burst `didChange` не блокирует интерактивный completion
- **GIVEN** пользователь быстро редактирует документ, и клиент отправляет серию `didChange`
- **WHEN** клиент запрашивает completion в процессе ввода
- **THEN** система обрабатывает запрос через per-file event-driven orchestrator без блокирующего ожидания завершения всех предыдущих интерактивных задач
- **AND** completion возвращается в bounded интерактивном времени

#### Scenario: Bounded queue предотвращает неограниченный рост backlog
- **GIVEN** для одного файла приходит burst событий выше устойчивой пропускной способности
- **WHEN** очередь orchestrator достигает лимита
- **THEN** policy переполнения сохраняет актуальные latest события для completion
- **AND** система не накапливает неограниченный per-file backlog

#### Scenario: Warm completion укладывается в SLO rollout-гейтов
- **GIVEN** фиксированный warm-профиль нагрузки (включая conf_big smoke) и включённый event-driven режим
- **WHEN** система собирает observability snapshot для интерактивного completion
- **THEN** `completion_duration_ms` p95 MUST быть не выше 1500ms
- **AND** `intellisense_v2_wait_for_file_version_completion_ms` p95 MUST быть не выше `(interactive_wait_budget_ms + 20ms)`
- **AND** `intellisense_v2_runtime_queue_wait_interactive_ms` p95 MUST быть не выше `(interactive_wait_budget_ms + 250ms)`

### Requirement: Event-driven completion соблюдает deterministic ordering, latest-wins и cancellation propagation (MUST)
Система MUST обеспечивать детерминированный порядок обработки событий в рамках одного документа и MUST использовать latest-wins политику для интерактивных completion запросов.

Каждый `CompletionRequest` MUST иметь monotonic `request_epoch` в рамках файла. Публикация completion ответа MUST выполняться только для актуального `request_epoch`.

Устаревшие completion запросы MUST отменяться до тяжёлых стадий вычисления (минимум: `snapshot`, `ir`, `collect`, `rank`, `format` checkpoints), если они потеряли актуальность относительно более новой ревизии/контекста.

`Cancel(request_id)` MUST доходить до orchestrator и MUST останавливать дальнейшее продвижение отменённого запроса между stage-checkpoints.

#### Scenario: Устаревший completion не конкурирует с актуальным запросом
- **GIVEN** клиент отправил completion для ревизии `N`, затем `didChange` до `N+1` и новый completion для `N+1`
- **WHEN** orchestrator планирует исполнение интерактивных задач
- **THEN** completion для `N+1` имеет приоритет как актуальный latest-wins запрос
- **AND** устаревший запрос для `N` не потребляет интерактивный бюджет после признания его неактуальным

#### Scenario: Отмена completion прерывает дальнейшие тяжёлые стадии
- **GIVEN** completion request уже запущен и клиент отправил `Cancel(request_id)`
- **WHEN** orchestrator обрабатывает отмену
- **THEN** запрос не продвигается дальше ближайшего stage-checkpoint
- **AND** отменённый запрос не публикует поздний пользовательский completion-ответ

### Requirement: Event-driven режим имеет mode-based rollout и безопасный rollback (MUST)
Система MUST поддерживать mode-based feature flag для event-driven completion с фиксированными значениями `off`, `shadow`, `canary`, `on`.

Семантика mode:
- `off`: пользовательские ответы формируются legacy/runtime-centric путём;
- `shadow`: event-driven путь исполняется для сравнения метрик/паритета, но пользовательский ответ остаётся legacy;
- `canary`: event-driven путь используется для части трафика по rollout policy;
- `on`: event-driven путь является default.

Система MUST сохранять безопасный rollback к legacy/runtime-centric пути переключением mode без изменения пользовательских editor settings.

Система MUST публиковать observability-сигналы, достаточные для сравнения режимов (latency/error/incomplete/cancel/stale metrics), включая mode-aware low-cardinality разрез.
Система MUST обеспечивать operation-scoped stage attribution для completion-контура (включая `parse_result_query`) в drilldown-метриках.

#### Scenario: Rollout и rollback выполняются переключением mode
- **GIVEN** event-driven completion включён в `canary` mode
- **WHEN** наблюдаются регрессии по интерактивным метрикам
- **THEN** команда может переключить mode в `off` и вернуться на legacy/runtime-centric путь
- **AND** клиентский контракт completion продолжает работать без ручных изменений настроек пользователя

#### Scenario: Shadow mode не влияет на пользовательский ответ
- **GIVEN** активирован `shadow` mode
- **WHEN** выполняется completion запрос
- **THEN** event-driven путь исполняется для сравнения telemetry/parity
- **AND** user-facing completion response возвращается из legacy/runtime-centric пути

#### Scenario: Observability позволяет формально сравнить legacy и event-driven режимы
- **GIVEN** один и тот же warm-профиль выполнен в режимах `off`/`shadow`/`canary`/`on`
- **WHEN** собраны метрики этапов completion-контура
- **THEN** метрики drilldown включают operation-scoped значения для `runtime_wait_for_file_version`, `runtime_snapshot_with_deps`, `ir_query`, `parse_result_query`
- **AND** mode-aware разрез позволяет формально оценить pass/fail по rollout SLO-гейтам
