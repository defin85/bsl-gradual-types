## ADDED Requirements

### Requirement: Completion v2 использует event-driven orchestration интерактивного пути (MUST)
Система MUST обрабатывать интерактивный путь completion через явную event-driven orchestration модель (`didChange`, completion request, cancel), чтобы исключить блокирующую сериализацию всего hot path и сохранять предсказуемое поведение при burst-нагрузке.

Система MUST ограничивать интерактивный tail latency под warm-нагрузкой с измеримыми SLO-гейтами rollout.

#### Scenario: Burst `didChange` не блокирует интерактивный completion
- **GIVEN** пользователь быстро редактирует документ, и клиент отправляет серию `didChange`
- **WHEN** клиент запрашивает completion в процессе ввода
- **THEN** система обрабатывает запрос через event-driven orchestrator без блокирующего ожидания завершения всех предыдущих интерактивных задач
- **AND** completion возвращается в bounded интерактивном времени

#### Scenario: Warm completion укладывается в SLO rollout-гейтов
- **GIVEN** фиксированный warm-профиль нагрузки (включая conf_big smoke) и включённый event-driven режим
- **WHEN** система собирает observability snapshot для интерактивного completion
- **THEN** `completion_duration_ms` p95 MUST быть не выше 1500ms
- **AND** `intellisense_v2_wait_for_file_version_completion_ms` p95 MUST быть не выше `(interactive_wait_budget_ms + 20ms)`
- **AND** `intellisense_v2_runtime_queue_wait_interactive_ms` p95 MUST быть не выше `(interactive_wait_budget_ms + 250ms)`

### Requirement: Event-driven completion соблюдает deterministic ordering и latest-wins semantics (MUST)
Система MUST обеспечивать детерминированный порядок обработки событий в рамках одного документа и MUST использовать latest-wins политику для интерактивных completion запросов.

Устаревшие completion запросы MUST отменяться до тяжёлых стадий вычисления, если они потеряли актуальность относительно более новой ревизии/контекста.

#### Scenario: Устаревший completion не конкурирует с актуальным запросом
- **GIVEN** клиент отправил completion для ревизии `N`, затем `didChange` до `N+1` и новый completion для `N+1`
- **WHEN** orchestrator планирует исполнение интерактивных задач
- **THEN** completion для `N+1` имеет приоритет как актуальный latest-wins запрос
- **AND** устаревший запрос для `N` не потребляет интерактивный бюджет после признания его неактуальным

### Requirement: Event-driven режим имеет управляемый rollout и безопасный rollback (MUST)
Система MUST поддерживать включение event-driven completion режима через feature flag и MUST сохранять безопасный rollback к legacy/runtime-centric пути без изменения пользовательских editor settings.

Система MUST публиковать observability-сигналы, достаточные для сравнения режимов (latency/error/incomplete/cancel/stale metrics) во время rollout.
Система MUST обеспечивать operation-scoped stage attribution для completion-контура (включая `parse_result_query`) в drilldown-метриках.

#### Scenario: Rollout и rollback выполняются переключением режима
- **GIVEN** event-driven completion включён только для canary-конфигурации
- **WHEN** наблюдаются регрессии по интерактивным метрикам
- **THEN** команда может отключить event-driven режим feature-flag'ом и вернуться на legacy/runtime-centric путь
- **AND** клиентский контракт completion продолжает работать без ручных изменений настроек пользователя

#### Scenario: Observability позволяет сравнить legacy и event-driven режимы
- **GIVEN** один и тот же warm-профиль выполнен в legacy/runtime-centric и event-driven режимах
- **WHEN** собраны метрики этапов completion-контура
- **THEN** метрики drilldown включают operation-scoped значения для `runtime_wait_for_file_version`, `runtime_snapshot_with_deps`, `ir_query`, `parse_result_query`
- **AND** на их основе можно формально оценить pass/fail по rollout SLO-гейтам
