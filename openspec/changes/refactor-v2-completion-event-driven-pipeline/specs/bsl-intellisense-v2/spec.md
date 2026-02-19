## ADDED Requirements

### Requirement: Completion v2 использует event-driven orchestration интерактивного пути (MUST)
Система MUST обрабатывать интерактивный путь completion через явную event-driven orchestration модель (`didChange`, completion request, cancel), чтобы исключить блокирующую сериализацию всего hot path и сохранять предсказуемое поведение при burst-нагрузке.

#### Scenario: Burst `didChange` не блокирует интерактивный completion
- **GIVEN** пользователь быстро редактирует документ, и клиент отправляет серию `didChange`
- **WHEN** клиент запрашивает completion в процессе ввода
- **THEN** система обрабатывает запрос через event-driven orchestrator без блокирующего ожидания завершения всех предыдущих интерактивных задач
- **AND** completion возвращается в bounded интерактивном времени

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

#### Scenario: Rollout и rollback выполняются переключением режима
- **GIVEN** event-driven completion включён только для canary-конфигурации
- **WHEN** наблюдаются регрессии по интерактивным метрикам
- **THEN** команда может отключить event-driven режим feature-flag'ом и вернуться на legacy/runtime-centric путь
- **AND** клиентский контракт completion продолжает работать без ручных изменений настроек пользователя
