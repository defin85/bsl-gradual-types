# mcp-bsl-agent Specification

## Purpose
Спецификация фиксирует контракт MCP‑сервера `bsl-agent` (stdio) для получения семантического контекста по BSL‑проекту локально (local-first), включая требования к read-only доступу к workspace, совместному использованию дискового кэша (LSP + MCP), детерминизму выдачи и `context_pack`/`context_expand`.
## Requirements
### Requirement: MCP server `bsl-agent` (stdio) для семантики проекта
Система SHALL предоставлять локальный MCP‑сервер `bsl-agent` по stdio, доступный для MCP‑клиентов (IDE/CLI), и реализующий lifecycle сессии, совместимый с асинхронной инициализацией и выполнением семантических операций.

Система SHALL обеспечивать, что `workspace_open` является “быстрым” tool-call (не выполняет тяжёлую инициализацию синхронно) и возвращает идентификатор сессии и идентификатор startup‑job (если требуется инициализация).

#### Scenario: Открытие сессии без блокировки инициализацией
- **GIVEN** локальный workspace путь добавлен в roots и указаны входы платформы/конфигурации
- **WHEN** клиент вызывает `workspace_open`
- **THEN** сервер возвращает `session_id`, `startup_job_id` и `ready=false` без длительного ожидания завершения парсинга/индексации

### Requirement: Read-only для workspace и sandbox чтения файлов
Система SHALL не модифицировать файлы проекта в roots (никаких write/patch) и SHALL ограничивать доступ к FS набором roots (sandbox), предотвращая path traversal и чтение вне roots.

Примечание: запись допускается только в директорию локального кэша (вне roots) и только для производных артефактов.

#### Scenario: Запрос к файлу вне roots запрещён
- **GIVEN** сессия открыта с roots
- **WHEN** клиент пытается запросить документ по пути вне roots
- **THEN** сервер возвращает ошибку `INVALID_PARAMS` (или эквивалентную) и не читает файл

### Requirement: Локальный кэш для платформы/конфигурации/AST (DiskCache)
Система SHALL поддерживать локальный дисковый кэш для тяжёлых артефактов (platform docs/config metadata/AST) и SHALL управляться переменными окружения, совместимыми с существующим кэшем проекта (`BSL_CACHE_DIR`, `BSL_CACHE_DISABLE`, `BSL_CACHE_STRICT_FINGERPRINT` и др.).

`platform_docs_archive` SHALL принимать как путь к файлу документации (например, `.hbk`/архив синтаксис‑помощника), так и путь к директории с распакованной документацией.

#### Scenario: Включённый кэш создаёт артефакты в `BSL_CACHE_DIR`
- **GIVEN** `BSL_CACHE_DIR` указывает на пустую временную директорию и `BSL_CACHE_DISABLE` не задан
- **WHEN** клиент открывает сессию с `platform_docs_archive` и/или `configuration_path` и дожидается готовности через `workspace_status`
- **THEN** в `BSL_CACHE_DIR` появляются артефакты кэша (manifest + payload) для соответствующих входных данных

#### Scenario: Отключённый кэш не читает и не пишет артефакты
- **GIVEN** `BSL_CACHE_DIR` указывает на пустую временную директорию и `BSL_CACHE_DISABLE=1`
- **WHEN** клиент открывает сессию с `platform_docs_archive` и/или `configuration_path`
- **THEN** система не создаёт и не использует артефакты дискового кэша (каталог остаётся пустым, либо без новых записей)

### Requirement: Совместное использование кэша несколькими процессами (LSP + MCP)
Система SHALL безопасно разделять один `DiskCache` между несколькими процессами (например, LSP сервером и `bsl-agent`), предотвращая повреждение артефактов при одновременной сборке/записи.

#### Scenario: Одновременная сборка одного ключа не повреждает кэш
- **GIVEN** два процесса используют один и тот же `BSL_CACHE_DIR` и один и тот же ключ кэша
- **WHEN** оба процесса одновременно вызывают операцию “получить или построить” (get-or-build) для этого ключа
- **THEN** артефакт в кэше остаётся валидным, и второй процесс получает либо готовый результат из кэша, либо корректно ждёт завершения записи

#### Scenario: Cleanup/eviction не удаляет entry под активным lock
- **GIVEN** один процесс удерживает per‑key `.lock` на cache entry
- **WHEN** другой процесс запускает cleanup/eviction (TTL/size) для дискового кэша
- **THEN** entry не удаляется, пока lock удерживается (entry пропускается), и взаимное исключение не нарушается

### Requirement: Unsaved buffers через ad-hoc snapshot и session overlay
Система SHALL поддерживать unsaved тексты как (1) ad-hoc snapshot для одного вызова и (2) session overlay для `scope=hot`.

#### Scenario: Overlay меняет ревизию анализа
- **GIVEN** сессия открыта и `analysis_revision = N`
- **WHEN** клиент вызывает `workspace_documents_set` с `FileRef.text`
- **THEN** сервер возвращает `analysis_revision = N+1`, и последующие семантические ответы ссылаются на новую ревизию

### Requirement: Семантические tools (MVP)
Система SHALL предоставлять семантические MCP tools только в асинхронной форме:
`bsl_diagnostics_start`, `bsl_symbol_search_start`, `bsl_type_at_position_start`, `bsl_members_start`, `bsl_definition_start`, `bsl_references_start`, `context_pack_start`, `context_expand_start`.

Каждый `*_start` tool SHALL возвращать `job_id` (и MAY возвращать `recommended_poll_ms`) и SHALL NOT возвращать финальный результат синхронно.

#### Scenario: Асинхронное получение диагностики по проекту
- **GIVEN** сессия открыта и `workspace_status.ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` со `scope=project`, затем опрашивает `job_status` до завершения, затем вызывает `job_result`
- **THEN** сервер возвращает diagnostics и `analysis_revision`, а `job_status.progress.percent` монотонно приближается к 100

### Requirement: Детерминизм выдачи и стабильные идентификаторы
Система SHALL обеспечивать детерминизм: одинаковый snapshot документов → одинаковые ответы (порядок и ID), а идентификаторы SHALL быть стабильны внутри одного `analysis_revision`.

#### Scenario: Повторный вызов возвращает те же результаты
- **GIVEN** snapshot документов не изменился
- **WHEN** клиент дважды вызывает один и тот же tool с одинаковыми параметрами
- **THEN** сервер возвращает одинаковые результаты и порядок, и те же ID

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

### Requirement: Read-only MCP tool `ui_url` для получения URL локального HTTP UI
Система SHALL предоставлять read-only MCP tool `ui_url`, который позволяет MCP-клиенту получить адрес и порт локального HTTP UI текущего инстанса `bsl-agent`.

Tool `ui_url` SHALL НЕ модифицировать никакое состояние и SHALL НЕ запускать HTTP UI: он только возвращает уже доступный URL (если UI включён и успешно стартовал).

Формат ответа SHALL включать:
- `enabled: bool`
- `ui_url: string | null` (вида `http://localhost:<port>`)

#### Scenario: UI включён, tool возвращает URL
- **GIVEN** `bsl-agent` запущен с включённым HTTP UI (например, `BSL_AGENT_HTTP_ADDR=127.0.0.1:0`) и UI успешно стартовал
- **WHEN** MCP-клиент вызывает tool `ui_url`
- **THEN** tool возвращает `enabled=true` и `ui_url` вида `http://localhost:<port>`

#### Scenario: UI выключен, tool не падает
- **GIVEN** `bsl-agent` запущен без HTTP UI
- **WHEN** MCP-клиент вызывает tool `ui_url`
- **THEN** tool возвращает `enabled=false` и `ui_url=null`

### Requirement: Read-only HTTP UI для диагностики MCP состояния (единый SPA)
Система SHALL предоставлять опциональный локальный HTTP UI для `bsl-agent`, предназначенный для разработчиков, чтобы визуально проверить состояние MCP (сессии, jobs, кэш, загрузка platform docs/config).

Система SHALL использовать существующий UI‑артефакт проекта (SPA из `frontend → target/site`) как единую точку ответственности UI, без введения отдельного “второго” UI для MCP.

HTTP UI для `bsl-agent` SHALL быть **строго read-only**: SHALL не предоставлять mutating endpoints (POST/PUT/PATCH/DELETE) и SHALL не модифицировать workspace roots.

HTTP UI SHALL быть выключен по умолчанию и SHALL быть доступен только на `127.0.0.1` (localhost-only). Попытка привязки к `0.0.0.0` SHALL быть отвергнута как ошибка конфигурации.

#### Scenario: Включённый UI поднимается локально и отдаёт SPA
- **GIVEN** запущен `bsl-agent` с включённым HTTP UI и корректным путём к `target/site`
- **WHEN** разработчик открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт SPA (fallback на `index.html`) и UI отображается в браузере

#### Scenario: UI не предоставляет write endpoints
- **GIVEN** включён HTTP UI `bsl-agent`
- **WHEN** клиент делает `POST` (или `PUT/PATCH/DELETE`) запрос к `/api/mcp/status`
- **THEN** сервер возвращает `405` (или `404`) и не изменяет никакое состояние

### Requirement: Capability detection режима UI (web-server vs mcp-agent)
Система SHALL предоставить read-only endpoint `GET /api/mcp/status`, который позволяет UI детектировать backend режим и корректно деградировать.

`bsl-web-server` SHALL реализовать совместимый `GET /api/mcp/status`, возвращающий `supported=false` и `mode=web-server`, чтобы единый SPA мог одинаково работать как в web-server, так и в mcp-agent окружении.

#### Scenario: UI переключается в MCP режим
- **GIVEN** UI загружен из `bsl-agent`
- **WHEN** UI выполняет `GET /api/mcp/status` и получает `supported=true` и `mode=mcp-agent`
- **THEN** UI показывает read-only “MCP Dashboard” и не вызывает web-server mutating API (например `POST /api/snapshot/reload`)

#### Scenario: UI корректно деградирует в web-server режиме
- **GIVEN** UI загружен из `bsl-web-server`
- **WHEN** UI выполняет `GET /api/mcp/status` и получает `supported=false` и `mode=web-server`
- **THEN** UI не показывает MCP-дашборд (или показывает “недоступно”), и продолжает работу в web-server режиме

### Requirement: Runtime registry для discovery HTTP UI (multi-instance в одном `BSL_CACHE_DIR`)
Система SHALL поддерживать “runtime discovery registry” для HTTP UI `bsl-agent`, чтобы клиент мог узнать фактический адрес и порт UI (включая случай автопорта `127.0.0.1:0`) без парсинга логов.

Когда HTTP UI включён (например, через `BSL_AGENT_HTTP_ADDR`) и успешно привязан (bind), `bsl-agent` SHALL записывать registry запись с фактическим `ui_url` в директории состояния, производной от `BSL_CACHE_DIR` (state root), так чтобы несколько параллельных инстансов в одном `BSL_CACHE_DIR` не конфликтовали между собой.

#### Scenario: Инстанс с автопортом записывает фактический порт в registry
- **GIVEN** запущен `bsl-agent` с `BSL_AGENT_HTTP_ADDR=127.0.0.1:0` и HTTP UI успешно стартовал
- **WHEN** процесс завершил bind
- **THEN** в state root появляется registry запись, содержащая `ui_url` вида `http://localhost:<port>` с фактическим портом

### Requirement: CLI discovery через `bsl-agent ui ...`
Система SHALL предоставлять CLI сабкоманды в бинарнике `bsl-agent` для discovery HTTP UI:
- `bsl-agent ui list` (список кандидатов),
- `bsl-agent ui url` (получить URL одного инстанса).

`bsl-agent ui url` SHALL печатать plain `http://localhost:<port>` (без лишнего текста), чтобы вывод мог использоваться в скриптах.

#### Scenario: Единственный инстанс возвращает URL
- **GIVEN** в registry есть ровно один “живой” инстанс HTTP UI
- **WHEN** пользователь запускает `bsl-agent ui url`
- **THEN** команда печатает `http://localhost:<port>` и завершается успешно

### Requirement: Безопасное поведение при неоднозначности (ошибка при >1)
Если в registry найдено более одного “живого” инстанса и пользователь не задал селектор, `bsl-agent ui url` SHALL завершаться ошибкой (без выбора “по умолчанию”) и SHALL печатать список кандидатов для уточнения.

Система SHALL поддерживать селектор `--roots <path>`, который выбирает инстанс по точному совпадению строки root среди `roots[]`, полученных из `GET /api/mcp/sessions`.

#### Scenario: Несколько инстансов без селектора приводят к ошибке
- **GIVEN** в registry есть два “живых” инстанса HTTP UI
- **WHEN** пользователь запускает `bsl-agent ui url` без селекторов
- **THEN** команда завершается ошибкой и печатает список кандидатов (например, `instance_id/pid/ui_url`)

#### Scenario: Селектор `--roots` выбирает нужный инстанс по точному совпадению
- **GIVEN** в registry есть два “живых” инстанса HTTP UI и они обслуживают разные `roots[]`
- **WHEN** пользователь запускает `bsl-agent ui url --roots <root>`
- **THEN** команда выбирает инстанс, у которого в `/api/mcp/sessions` есть `roots[]`, содержащий строку `<root>` в точном совпадении

### Requirement: Embedded SPA для HTTP UI `bsl-agent` (работает без внешней статики)
Система SHALL встраивать (embed) артефакт SPA (собранный `frontend → target/site`) внутрь бинарника `bsl-agent` и SHALL раздавать его через HTTP UI, чтобы UI мог работать без внешних файлов статики.

#### Scenario: UI работает без `BSL_AGENT_HTTP_STATIC_DIR`
- **GIVEN** `bsl-agent` собран со встроенным SPA и запущен с включённым HTTP UI
- **WHEN** клиент открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт `index.html` и остальные ассеты из embedded набора

### Requirement: `BSL_AGENT_HTTP_STATIC_DIR` имеет приоритет над embedded
Если `BSL_AGENT_HTTP_STATIC_DIR` задан и указывает на существующую директорию, `bsl-agent` SHALL раздавать статику с диска из этой директории, даже если embedded статика присутствует.

#### Scenario: Внешняя статика перекрывает embedded
- **GIVEN** `bsl-agent` собран со встроенным SPA
- **AND** `BSL_AGENT_HTTP_STATIC_DIR` указывает на директорию со статикой
- **WHEN** клиент открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт файлы статики с диска из `BSL_AGENT_HTTP_STATIC_DIR`

### Requirement: Build-time ошибка при отсутствии `target/site`
Система SHALL завершать сборку `bsl-agent` с понятной ошибкой, если артефакт SPA для embed отсутствует (например, не существует `target/site/index.html`).

#### Scenario: Сборка без SPA завершается ошибкой
- **GIVEN** `target/site` отсутствует
- **WHEN** выполняется сборка `bsl-agent`
- **THEN** сборка завершается ошибкой с сообщением о необходимости сначала собрать `frontend`

### Requirement: Single-session политика MCP (не более одной workspace-сессии на процесс)
Система SHALL запрещать одновременную работу более чем одной workspace-сессии в одном процессе `bsl-agent`.

Если в процессе уже существует активная workspace-сессия, повторный вызов `workspace_open` SHALL обрабатываться так:
- Если параметры вызова совпадают с уже открытой сессией (roots + входы platform/config), `workspace_open` SHALL быть идемпотентным и SHALL возвращать `session_id` этой существующей сессии (не создавая новую).
- Если параметры отличаются, `workspace_open` SHALL отклоняться как `INVALID_PARAMS` (HTTP 400 или эквивалент) с понятным сообщением (например, “only one session is allowed; close the existing session first”).

Это ограничение вводится для устранения неоднозначности выбора “текущей” сессии для parity UI и упрощения поведения MCP-клиентов в модели per-agent.

#### Scenario: Открытие первой сессии разрешено
- **GIVEN** в процессе `bsl-agent` нет активных workspace-сессий
- **WHEN** клиент вызывает `workspace_open` с `roots` и входами platform/config
- **THEN** сервер возвращает `session_id` и начинает startup (job), как обычно

#### Scenario: Повторный `workspace_open` с теми же параметрами идемпотентен
- **GIVEN** в процессе `bsl-agent` уже существует активная workspace-сессия, открытая с некоторыми `roots` и входами platform/config
- **WHEN** клиент вызывает `workspace_open` повторно с теми же `roots` и теми же входами platform/config
- **THEN** сервер возвращает тот же `session_id` и не создаёт новую сессию

#### Scenario: Попытка открыть вторую сессию с другими параметрами отклоняется
- **GIVEN** в процессе `bsl-agent` уже существует активная workspace-сессия
- **WHEN** клиент вызывает `workspace_open` повторно с другими `roots` или другими входами platform/config (без предварительного `workspace_close`)
- **THEN** сервер возвращает `INVALID_PARAMS` (HTTP 400 или эквивалент) и сообщение о том, что разрешена только одна сессия

### Requirement: Parity HTTP API для UI в режиме `bsl-agent` (types/search/metrics)
Система SHALL предоставлять read-only parity HTTP API в `bsl-agent` для отображения тех же экранов UI, что и `bsl-web-server`, но на данных workspace-сессии MCP.

Parity API SHALL предоставляться в namespace `/api/mcp/*` и SHALL возвращать те же DTO, что и web-server:
- `GET /api/mcp/types` → `AnalysisResultDto`
- `GET /api/mcp/search` → `AnalysisResultDto`
- `GET /api/mcp/metrics` → `MetricsDto`

Параметры запросов SHALL соответствовать web-server API (пагинация/фильтры/поиск). Parity API MUST быть строго read-only.

#### Scenario: Получение типов из MCP сессии через parity API
- **GIVEN** существует ровно одна MCP сессия с `ready=true`
- **WHEN** UI делает `GET /api/mcp/types?page=1&limit=50`
- **THEN** сервер возвращает `200` и `AnalysisResultDto`, отражающий типы, доступные в этой MCP сессии

#### Scenario: Поиск типов через parity API
- **GIVEN** существует ровно одна MCP сессия с `ready=true`
- **WHEN** UI делает `GET /api/mcp/search?q=ТаблицаЗначений`
- **THEN** сервер возвращает `200` и `AnalysisResultDto` с результатами поиска для этой MCP сессии

### Requirement: Правило выбора сессии для parity API (ровно одна ready без sessionId)
Система SHALL применять единое правило выбора workspace-сессии для parity API:
- Если `sessionId` передан, сервер SHALL использовать указанную сессию.
- Если `sessionId` не передан, сервер SHALL требовать, чтобы существовала ровно одна сессия с `ready=true`, и SHALL использовать её.

Если правило не выполнено, сервер SHALL возвращать `INVALID_PARAMS` (HTTP 400) с понятным сообщением:
- при 0 ready: “no ready sessions” (или эквивалент)
- при >1 ready: “exactly one ready session is required” (или эквивалент)

Сервер SHALL отклонять запросы к parity API для не-ready сессии (`ready=false`) как `INVALID_PARAMS` (HTTP 400).

#### Scenario: Нет ready сессии — parity API отклоняется
- **GIVEN** сессий с `ready=true` нет (startup ещё не завершён)
- **WHEN** UI делает `GET /api/mcp/types` без `sessionId`
- **THEN** сервер возвращает `400 INVALID_PARAMS` и сообщение о том, что нет ready сессий

#### Scenario: Несколько ready сессий — parity API отклоняется
- **GIVEN** существует две или более сессии с `ready=true`
- **WHEN** UI делает `GET /api/mcp/search?q=Документы` без `sessionId`
- **THEN** сервер возвращает `400 INVALID_PARAMS` и сообщение о необходимости ровно одной ready сессии

### Requirement: UI в MCP режиме использует parity API и сохраняет MCP диагностику
Единый SPA (`frontend → target/site`) в MCP режиме SHALL:
- показывать те же экраны (Dashboard/Карточки/Таблица/Граф + поиск/фильтры), что и в `bsl-web-server`;
- использовать parity API `/api/mcp/types|search|metrics` и `/api/mcp/deps/meta`;
- сохранять доступ к MCP диагностике (сессии/jobs) как к MCP-специфическому экрану/разделу;
- оставаться строго read-only (не вызывать mutating endpoints).

Если условие “ровно одна ready сессия” не выполнено, UI SHALL показывать понятную инструкцию и SHALL не делать parity вызовы без `sessionId`.

#### Scenario: UI показывает web-server экраны в MCP режиме при одной ready сессии
- **GIVEN** UI загружен из `bsl-agent` и `GET /api/mcp/status` возвращает `mode=mcp-agent`
- **AND** существует ровно одна сессия с `ready=true`
- **WHEN** пользователь открывает вкладку “Таблица”
- **THEN** UI отображает таблицу типов, используя `GET /api/mcp/types`

#### Scenario: UI показывает инструкцию при 0 или >1 ready сессии
- **GIVEN** UI загружен из `bsl-agent`
- **AND** ready-сессий 0 или >1
- **WHEN** UI пытается загрузить данные для таблицы типов
- **THEN** UI показывает инструкцию “оставьте ровно одну ready сессию” и не делает parity вызовы без `sessionId`

### Requirement: On-demand справка/примеры для MCP tool-ов (`mcp_help`)
Система SHALL предоставлять read-only tool `mcp_help` (или эквивалент), который позволяет MCP‑клиенту получить канонические примеры вызовов и правила форматирования параметров **по запросу**, чтобы не раздувать `tools/list`.

`mcp_help` SHALL поддерживать:
- общий quickstart сценарий (workspace_open → wait ready → documents_set → diagnostics/search → job_result),
- выдачу 2–3 типичных примеров payload’ов по `tool_name`,
- краткие правила для multi-root путей и scope,
- список типичных причин `INVALID_PARAMS`.

#### Scenario: Клиент получает примеры payload’ов для конкретного tool-а
- **GIVEN** MCP клиент подключён к `bsl-agent`
- **WHEN** клиент вызывает `mcp_help` с `tool_name="workspace_documents_set"`
- **THEN** сервер возвращает короткий набор примеров payload’ов и пояснение ключевых ограничений (например, version required with text)

### Requirement: Описания tool-ов в `tools/list` краткие и однозначные
Система SHALL обеспечивать, что `tools/list` содержит `description`, достаточный для однозначного использования tool-ов без “угадывания”, но при этом остаётся компактным (без многострочных JSON-примеров в каждом tool).

Описания tool-ов SHALL:
- быть однострочными (1 line),
- фиксировать ключевые форматы и ограничения (пути/roots, scope, позиция, version/text),
- для async tool-ов явно указывать паттерн `*_start → job_wait/job_result`,
- при наличии on-demand справки упоминать `mcp_help` как источник примеров.

#### Scenario: `tools/list` не содержит многострочных примеров в tool.description
- **GIVEN** клиент вызывает `tools/list`
- **WHEN** клиент читает `description` каждого tool-а
- **THEN** `description` является однострочным и не содержит встроенных многострочных примеров JSON

