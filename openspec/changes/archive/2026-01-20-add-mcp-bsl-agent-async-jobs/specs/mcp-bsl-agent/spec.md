# Spec Delta: mcp-bsl-agent — async jobs + progress + persist/resume

## MODIFIED Requirements

### Requirement: MCP server `bsl-agent` (stdio) для семантики проекта
Система SHALL предоставлять локальный MCP‑сервер `bsl-agent` по stdio, доступный для MCP‑клиентов (IDE/CLI), и реализующий lifecycle сессии, совместимый с асинхронной инициализацией и выполнением семантических операций.

Система SHALL обеспечивать, что `workspace_open` является “быстрым” tool-call (не выполняет тяжёлую инициализацию синхронно) и возвращает идентификатор сессии и идентификатор startup‑job (если требуется инициализация).

#### Scenario: Открытие сессии без блокировки инициализацией
- **GIVEN** локальный workspace путь добавлен в roots и указаны входы платформы/конфигурации
- **WHEN** клиент вызывает `workspace_open`
- **THEN** сервер возвращает `session_id`, `startup_job_id` и `ready=false` без длительного ожидания завершения парсинга/индексации

### Requirement: Семантические tools (MVP)
Система SHALL предоставлять семантические MCP tools только в асинхронной форме:
`bsl_diagnostics_start`, `bsl_symbol_search_start`, `bsl_type_at_position_start`, `bsl_members_start`, `bsl_definition_start`, `bsl_references_start`, `context_pack_start`, `context_expand_start`.

Каждый `*_start` tool SHALL возвращать `job_id` (и MAY возвращать `recommended_poll_ms`) и SHALL NOT возвращать финальный результат синхронно.

#### Scenario: Асинхронное получение диагностики по проекту
- **GIVEN** сессия открыта и `workspace_status.ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` со `scope=project`, затем опрашивает `job_status` до завершения, затем вызывает `job_result`
- **THEN** сервер возвращает diagnostics и `analysis_revision`, а `job_status.progress.percent` монотонно приближается к 100

### Requirement: `context_pack` с жёстким бюджетом и дозапросом
Система SHALL предоставлять `context_pack_start` и `context_expand_start` как асинхронные tools, которые возвращают результат через `job_result`.

Требование по бюджету SHALL сохраняться: итоговый `context_pack` текст MUST быть строго в рамках `budget_chars`.

#### Scenario: Превышение бюджета приводит к явной обрезке (async)
- **GIVEN** `budget_chars` задан как жёсткий лимит
- **WHEN** клиент запускает `context_pack_start` и получает результат через `job_result`
- **THEN** сервер возвращает `truncated=true` и текст строго в рамках `budget_chars`

### Requirement: Интеграционные тесты MCP (stdio)
Система SHALL иметь интеграционные тесты, проверяющие асинхронный MCP контракт по stdio (initialize/tools/list/tools/call), включая:
- `workspace_open` → `workspace_status` polling до готовности,
- `*_start` → `job_status/job_wait` → `job_result`,
- persist/resume сценарий после рестарта процесса.

#### Scenario: Интеграционный тест проверяет async flow и resume
- **GIVEN** тест поднимает `bsl-agent` как процесс и открывает сессию
- **WHEN** тест завершает процесс и поднимает новый, затем вызывает `workspace_resume`
- **THEN** сессия восстанавливается, а завершённые результаты доступны через `job_result` или корректно перезапускаются

## ADDED Requirements

### Requirement: Job‑модель для долгих операций (status/wait/result/cancel)
Система SHALL предоставлять унифицированные tools управления job’ами:
`job_status`, `job_wait`, `job_result`, `job_cancel`.

Система SHALL хранить для job состояние (`queued|running|succeeded|failed|canceled|aborted_by_restart`), фазу (`phase`) и прогресс (`progress.percent` в диапазоне 0..100).

`job_wait` SHALL реализовывать long‑poll: ожидать до `timeout_ms` и возвращать обновлённый статус без передачи результата.

#### Scenario: Запуск job и ожидание через long‑poll
- **GIVEN** сессия открыта и готова к семантике
- **WHEN** клиент запускает `bsl_symbol_search_start`, затем вызывает `job_wait(timeout_ms=...)` несколько раз
- **THEN** клиент наблюдает изменение `job_status` (state/percent) и в конце получает `succeeded`

### Requirement: Прогресс startup через `workspace_status` и `job_status`
Система SHALL выполнять startup (загрузка platform docs/config + подготовка зависимостей) как job и SHALL экспонировать его прогресс через:
- `workspace_status` (агрегированно для текущей сессии),
- `job_status` по `startup_job_id`.

`workspace_status.ready` SHALL быть `true` только после завершения startup (либо успешного, либо завершившегося в деградированном режиме, если fallback допустим).

#### Scenario: Клиент получает понятный сигнал готовности
- **GIVEN** клиент получил `startup_job_id` из `workspace_open`
- **WHEN** клиент периодически вызывает `workspace_status`
- **THEN** `phase/progress.percent` отражают ход инициализации, а `ready` становится `true` при завершении

### Requirement: Persist/Resume состояния сессии и результатов
Система SHALL сохранять состояние сессии и job’ов на диск (persist), чтобы поддержать `workspace_resume` после рестарта процесса.

Система SHALL хранить как минимум:
- параметры сессии (roots + входы platform/config),
- текущий `analysis_revision`,
- состояния job’ов и результаты для завершённых job’ов (в пределах лимитов/TTL).

Незавершённые job’ы (`queued|running`) при рестарте SHALL помечаться как `aborted_by_restart` и SHALL требовать перезапуска через соответствующий `*_start`.

#### Scenario: Resume после рестарта процесса
- **GIVEN** ранее была открыта сессия и выполнен хотя бы один `*_start` до `succeeded`
- **WHEN** MCP процесс перезапускается и клиент вызывает `workspace_resume(session_id)`
- **THEN** сервер возвращает восстановленный статус сессии и предоставляет доступ к завершённым результатам через `job_result`

