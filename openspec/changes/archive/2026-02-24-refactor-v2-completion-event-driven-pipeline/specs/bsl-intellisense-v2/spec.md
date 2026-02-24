## ADDED Requirements

### Requirement: Completion v2 использует per-file event-driven orchestrator для интерактивного пути (MUST)
Система MUST обрабатывать интерактивный completion через per-file event-driven orchestrator (`dispatcher/actor`) с явными событиями `DidOpen`, `DidChange`, `CompletionRequest`, `Cancel`, `DidClose`.

В рамках данного change целевой production design MUST соответствовать только этой модели. Любая другая архитектурная схема MUST NOT становиться целевой реализацией.

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

### Requirement: Event envelope и ordering contract формализованы для per-file stream (MUST)
Каждое входящее событие per-file orchestrator MUST иметь envelope с полями:
- `file_id`;
- `file_seq` (monotonic, строго возрастающий в рамках файла);
- `received_at` (время постановки в orchestrator);
- typed `payload`.

Каждый `CompletionRequest` payload MUST включать:
- `request_id` (LSP request identifier);
- `request_epoch` (monotonic per-file epoch для latest-wins);
- `version_hint`;
- `trigger_mode`.

Каждый `CompletionRequest` MUST иметь monotonic `request_epoch` в рамках файла. Публикация completion ответа MUST выполняться только для актуального `request_epoch`.

Внутри одного `file_id` orchestrator MUST обрабатывать события детерминированно по `file_seq` и MUST NOT публиковать user-facing completion response для superseded epoch.

#### Scenario: Ответ публикуется только для latest epoch
- **GIVEN** по одному `file_id` отправлены два `CompletionRequest` с `request_epoch=10` и `request_epoch=11`
- **WHEN** обработка `epoch=10` завершилась позже `epoch=11`
- **THEN** пользовательский completion-ответ для `epoch=10` не публикуется
- **AND** публикуется только ответ для latest epoch (`epoch=11`)

### Requirement: Bounded queue и overflow policy для интерактивного completion детерминированы (MUST)
Per-file inbox MUST быть bounded и конфигурироваться runtime key `BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY` (значение MUST проходить clamp до безопасного диапазона).

Overflow policy MUST сохранять latest-wins semantics:
- pending `DidChange` для одного файла MUST коалесцироваться до latest revision;
- устаревшие pending `CompletionRequest` (меньший `request_epoch`) MUST вытесняться/отменяться до тяжёлых стадий;
- `Cancel(request_id)` MUST иметь приоритет доставки и MUST NOT теряться из-за переполнения очереди.

Система MUST NOT допускать неограниченный рост per-file backlog.

#### Scenario: Overflow не ломает latest-wins и не теряет cancel
- **GIVEN** очередь файла заполнена burst-событиями
- **WHEN** приходит `Cancel(request_id)` и более новый `CompletionRequest`
- **THEN** cancel доставляется до orchestrator
- **AND** более новый completion сохраняется как latest
- **AND** устаревшие completion не копятся без ограничений

### Requirement: Event-driven completion соблюдает latest-wins и cancellation propagation по stage checkpoints (MUST)
Устаревшие completion запросы MUST отменяться до тяжёлых стадий вычисления (минимум: `wait_for_file_version`, `snapshot_with_deps`, `ir_query`, `collect`, `rank`, `format`, `publish` checkpoints), если они потеряли актуальность относительно более новой ревизии/контекста.

`Cancel(request_id)` MUST доходить до orchestrator и MUST останавливать дальнейшее продвижение отменённого запроса между stage-checkpoints.

`$/cancelRequest` от LSP MUST маппиться в `Cancel(request_id)` через request-level registry (`request_id -> file_id/request_epoch/token`).

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

#### Scenario: LSP cancel преобразуется в orchestrator cancel event
- **GIVEN** LSP отправил `$/cancelRequest` для активного completion request
- **WHEN** adapter получает отмену
- **THEN** adapter публикует `Cancel(request_id)` в per-file orchestrator stream
- **AND** stage execution прекращается на ближайшем checkpoint

### Requirement: Event-driven режим имеет mode-based rollout и безопасный rollback (MUST)
Система MUST поддерживать mode-based feature flag для event-driven completion с фиксированными значениями `off`, `shadow`, `canary`, `on`.

Mode MUST задаваться runtime key `BSL_INTELLISENSE_V2_COMPLETION_MODE`.

Canary доля MUST задаваться runtime key `BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT` (`0..100`) и MUST маршрутизироваться детерминированно для воспроизводимого сравнения.

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

#### Scenario: Rollout в canary детерминирован
- **GIVEN** активирован `canary` mode и задан `BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT`
- **WHEN** выполняются повторные completion запросы для одного и того же deterministic routing ключа
- **THEN** решение маршрутизации (`legacy` или `event_driven`) стабильно и воспроизводимо

### Requirement: Observability контракт включает mode-aware измерение для rollout gates (MUST)
Drilldown observability completion-контура MUST включать low-cardinality измерение `mode` со значениями:
- `legacy`;
- `event_driven`;
- `shadow`.

Mode-aware метрики MUST быть доступны минимум для стадий:
- `runtime_wait_for_file_version`;
- `runtime_snapshot_with_deps`;
- `ir_query`;
- `parse_result_query`.

Система MUST обеспечивать формальные rollout pass/fail gates на mode-aware срезе:
- `completion_duration_ms` p95 `<= 1500ms`;
- `intellisense_v2_wait_for_file_version_completion_ms` p95 `<= interactive_wait_budget_ms + 20ms`;
- `intellisense_v2_runtime_queue_wait_interactive_ms` p95 `<= interactive_wait_budget_ms + 250ms`;
- `completion_cancelled_rate <= 0.10`;
- `completion_parity_drift_rate <= 0.01` (для `shadow`/`canary`);
- `member_access_terminal_empty_missing_ir_rate <= 0.005` (для `shadow`/`canary`).

#### Scenario: Observability позволяет формально сравнить legacy и event-driven режимы
- **GIVEN** один и тот же warm-профиль выполнен в режимах `off`/`shadow`/`canary`/`on`
- **WHEN** собраны метрики этапов completion-контура
- **THEN** метрики drilldown включают operation-scoped значения для `runtime_wait_for_file_version`, `runtime_snapshot_with_deps`, `ir_query`, `parse_result_query`
- **AND** mode-aware разрез позволяет формально оценить pass/fail по rollout SLO-гейтам
