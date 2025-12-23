# Roadmap: Архитектура прогресса парсинга (LSP WorkDoneProgress, multi-token, реиндексация “на лету”)

**Статус:** 🟡 В ПРОЦЕССЕ (multi-token и incremental update уже реализованы; остаются унификация прогресса старта, e2e тесты и UI Web API)  
**Приоритет:** HIGH  
**Цель:** сделать прогресс парсинга **единым и честным**: сервер — “источник истины”, проценты монотонны, расширение корректно отображает **несколько параллельных задач**, а реиндексация конфигурации “на лету” (после правок/пересборки) даёт понятный прогресс без дублей.

---

## Контекст (проверено по репозиторию)

### Что уже есть на сервере
- Startup загрузка типов использует `window/workDoneProgress/create` + `$/progress` + throttling и ETA: `backend/src/bin/lsp_server/server/language_server.rs:153`, `backend/src/bin/lsp_server/server/language_server.rs:190`.
- Команда `bsl.parseConfiguration` использует server-initiated WorkDoneProgress и завершает `End` **после** индексации BSL модулей: `backend/src/bin/lsp_server/commands/configuration.rs:282`, `backend/src/bin/lsp_server/commands/configuration.rs:293`, `backend/src/bin/lsp_server/commands/configuration.rs:382`.
- `bsl/incrementalUpdate` реализован как custom request и использует `LspWorkDoneReporter` с токеном `bsl-incremental-update-*`: `backend/src/bin/lsp_server/main.rs:111`, `backend/src/bin/lsp_server/commands/configuration.rs:398`, `backend/src/bin/lsp_server/progress_bridge.rs:92`.
- Детальный прогресс индексации модулей (X/Y) есть в parseConfiguration и Web API startup: `backend/src/bin/lsp_server/commands/configuration.rs:296`, `backend/src/system/system_coordinator/config_loader.rs:210`.
- `bsl/serverStatus` используется только для “loading/ready” статуса (отдельно от процентов): `backend/src/bin/lsp_server/types.rs:12`, `backend/src/bin/lsp_server/server/language_server.rs:157`.
- Web API endpoint `/api/startup/progress` доступен: `backend/src/presentation/web/router.rs:38`, `backend/src/presentation/web/handlers.rs:119`.

### Что уже есть в расширении
- Обработчик `$/progress` хранит состояние по token и корректно ведёт параллельные прогрессы: `vscode-extension/src/lsp/client/progress-handler.ts:27`, `vscode-extension/src/test/suite/progress-handler.test.ts:1`.
- Команды `buildIndex`/`incrementalUpdate` не используют локальный `withProgress`, а опираются на `$/progress` + status bar: `vscode-extension/src/commands/index-commands.ts:71`, `vscode-extension/src/commands/index-commands.ts:143`.
- Команда `parseConfiguration` не дублирует прогресс локальным `withProgress` (ориентир на `$/progress`): `vscode-extension/src/commands/parseConfiguration.ts:28`.

### Что отсутствует
- Полная унификация прогресса старта (форматирование/проценты всё ещё в `language_server.rs`, а не через общий `ProgressPlan`).
- UI/клиент для отображения `/api/startup/progress` (endpoint и тесты есть, UI нет).
- E2E/интеграционные тесты на прогресс для startup/parseConfiguration/incrementalUpdate (есть только unit).

---

## Принципы (источник истины)

1) **Сервер — источник истины** для прогресса CPU/IO-heavy операций (парсинг docs/config, индексация BSL, buildIndex, incremental update).  
2) **Единый транспорт прогресса**: server-initiated WorkDoneProgress (`$/progress`) + опционально `bsl/serverStatus` для статуса “loading/ready”.  
3) **Проценты монотонны** (никогда не уменьшаются).  
4) **Multi-token**: несколько задач прогресса одновременно допустимы; UI должен отображать их корректно (не один глобальный state).  
5) **Throttling** по времени (100–250ms) и “last update always delivered”.

---

## Milestones (вертикальные срезы)

### P1: Multi-token progress в расширении 🟢
**Цель:** `$/progress` отображает параллельные задачи корректно.

**Подход:**
- `Map<token, ProgressState>` вместо одного `state`.
- `begin/report/end` маршрутизируются по `params.token`.

**Критерии успеха:**
- Startup progress и progress команды `parseConfiguration` не конфликтуют.
- Прогресс реиндексации (см. P5) не “гасит” прогресс загрузки типов и наоборот.

**Проверка:**
- ✅ `Map<token, ProgressState>` в `$/progress` handler: `vscode-extension/src/lsp/client/progress-handler.ts:38`.
- ✅ unit-тест на независимое завершение токенов: `vscode-extension/src/test/suite/progress-handler.test.ts:1`.

---

### P2: Унифицированный progress bridge на сервере 🟡
**Цель:** убрать ручное форматирование/проценты из отдельных команд и сделать один “движок” прогресса.

**Подход:**
- Ввести `ProgressReporter` (trait) и `LspWorkDoneReporter` (адаптер).
- Ввести `ProgressPlan` (веса/диапазоны стадий) для `startup` / `parseConfiguration` / `buildIndex` / `incrementalUpdate`.

**Критерии успеха:**
- Внутренние loader’ы репортят `current/total/message`, проценты считаются централизованно.
- Проценты не “прыгают” и не доходят до 100% раньше фактического конца.

**Проверка:**
- ✅ `ProgressPlan` + `LspWorkDoneReporter` используются в parseConfiguration/buildIndex/incrementalUpdate: `backend/src/bin/lsp_server/progress_bridge.rs:45`, `backend/src/bin/lsp_server/commands/configuration.rs:499`.
- ❌ Startup ещё форматирует сообщения/ETA вручную в `language_server.rs` (часть логики вне общего планировщика).

---

### P3: Детальный прогресс по BSL-модулям (X/Y) 🟢
**Цель:** при индексации `*.bsl` модулей показывать прогресс по файлам.

**Подход:**
- Добавить опциональный callback/reporter в `index_configuration_bsl_modules`.
- До парсинга собрать список модулей → `total`.
- Репортить раз в N файлов/по throttling: `Indexed i/total: <path>`.

**Критерии успеха:**
- В `bsl.parseConfiguration` прогресс в стадии “Indexing configuration module methods (*.bsl)” отражает i/total.

**Проверка:**
- ✅ callback в `index_configuration_bsl_modules_with_progress_parallel`: `backend/src/data/loaders/config_bsl_modules/indexing.rs:212`.
- ✅ parseConfiguration репортит `Indexed i/total`: `backend/src/bin/lsp_server/commands/configuration.rs:296`.
- ✅ Web API startup пишет прогресс модулей: `backend/src/system/system_coordinator/config_loader.rs:210`.

---

### P4: Устранить дублирование прогресса в UI 🟢
**Цель:** не показывать два прогресса одновременно (Notification + Window) для одной операции.

**Подход:**
- Команды, которые инициируют серверные операции (buildIndex/incrementalUpdate), не рисуют локальный `withProgress` с условными инкрементами.
- Использовать `$/progress` как основной UI, а Notification оставить только для итогового сообщения.

**Проверка:**
- ✅ `parseConfiguration` уже так сделан: `vscode-extension/src/commands/parseConfiguration.ts:28`.
- ✅ `buildIndex` и `incrementalUpdate` используют только `$/progress` + status bar: `vscode-extension/src/commands/index-commands.ts:71`, `vscode-extension/src/commands/index-commands.ts:143`.

---

### P5: Реиндексация “на лету” (после правок конфигурации) 🟢
**Цель:** когда конфигурация/модули меняются на диске (выгрузка новой версии, пересборка, обновление расширения конфы), индекс обновляется без перезапуска, с понятным прогрессом.

**Сценарии:**
1) **Ручной инкрементальный апдейт** (команда): `bslAnalyzer.incrementalUpdate`.
2) **Авто-реиндексация**: при изменении файлов конфигурации (например `Configuration.xml`, `*/Ext/*.bsl`, `Forms/*/Form.xml`) расширение/сервер инициирует incremental update.

**Архитектурный выбор (реализовано):**
- Сервер получает custom request `bsl/incrementalUpdate` и внутри запускает пайплайн:
  - discovery diff → перечень затронутых объектов/модулей
  - обновление `TypeRepository`/`SignatureIndex` частями
  - инвалидирование кэшей/IR по затронутым файлам
- Прогресс идёт через отдельный WorkDoneProgress token `bsl-incremental-update-*`.

**Критерии успеха:**
- Команда `bslAnalyzer.incrementalUpdate` показывает прогресс через `$/progress` (а не локальный “условный”).
- Можно “переподхватить” изменения конфигурации без restart LSP.
- Прогресс отражает стадии (discover changes / reload types / reindex bsl modules / finalize).

**Проверка:**
- ✅ команда в расширении есть: `vscode-extension/src/commands/index-commands.ts:143`.
- ✅ сервер реализует `bsl/incrementalUpdate`: `backend/src/bin/lsp_server/main.rs:111`, `backend/src/bin/lsp_server/commands/configuration.rs:398`.

---

### P6: Тесты прогресса (unit/integration/e2e) 🟡
**Цель:** прогресс не ломается при рефакторинге и поддерживает multi-token.

**Критерии успеха:**
- Unit: расчёт процентов и монотонность (серверный progress engine).
- Integration: `$/progress` handler корректно маршрутизирует по token.
- E2E: smoke-тест на progress begin/report/end для startup + parseConfiguration + incrementalUpdate.

**Проверка:**
- ✅ unit-тесты: `backend/src/bin/lsp_server/progress_bridge.rs:197`, `vscode-extension/src/test/suite/progress-handler.test.ts:1`.
- ❌ нет интеграционных/e2e тестов на реальные progress-сессии.

---

### P7: Прогресс загрузки при старте Web API сервера 🟡
**Цель:** при запуске `bsl-web-server` прогресс загрузки типов/конфигурации виден пользователю не только в логах; проценты/стадии монотонны, а “источник истины” — сервер.

**Подход (MVP):**
- Ввести единый `StartupProgress` в `SystemCoordinator` (аналог progress engine для LSP), который пишет состояние в shared storage (например, `Arc<RwLock<...>>`).
- Добавить endpoint для чтения прогресса (polling): `GET /api/startup/progress` → `{ phase, current, total, percentage, message, done }`.
- Обновить Web UI (если включён) чтобы отображал этот прогресс (poll раз в 200–500ms) и корректно завершал после “индексации BSL модулей”.

**Критерии успеха:**
- При запуске с `--project-path` пользователь видит стадии: загрузка syntax-helper → discovery конфигураций → парсинг/линковка → индексация `*.bsl` → ready.
- `/api/startup/progress` возвращает `done=true` только после завершения индексации модулей.

**Проверка:**
- ✅ endpoint и handler: `backend/src/presentation/web/router.rs:38`, `backend/src/presentation/web/handlers.rs:119`.
- ✅ интеграционный тест: `backend/tests/startup_progress_endpoint_test.rs:1`.
- ❌ UI/polling для отображения прогресса не добавлен.

## Следующие шаги (минимальный путь)

1) P2: централизовать расчёт процентов и форматирование для startup (уйти от ручных сообщений/ETA в `language_server.rs`).
2) P6: добавить интеграционные/e2e тесты прогресса для startup + parseConfiguration + incrementalUpdate.
3) P7: добавить UI/polling для `/api/startup/progress` (если Web UI актуален).
