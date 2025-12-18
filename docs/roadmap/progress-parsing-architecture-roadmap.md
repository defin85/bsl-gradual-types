# Roadmap: Архитектура прогресса парсинга (LSP WorkDoneProgress, multi-token, реиндексация “на лету”)

**Статус:** 🟡 В ПРОЦЕССЕ (часть прогресса уже есть, но нет multi-token и нет прогресса для реиндексации во время работы)  
**Приоритет:** HIGH  
**Цель:** сделать прогресс парсинга **единым и честным**: сервер — “источник истины”, проценты монотонны, расширение корректно отображает **несколько параллельных задач**, а реиндексация конфигурации “на лету” (после правок/пересборки) даёт понятный прогресс без дублей.

---

## Контекст (проверено по репозиторию)

### Что уже есть на сервере
- Startup загрузка типов использует `window/workDoneProgress/create` + `$/progress` + throttling и ETA: `backend/src/bin/lsp_server/server/language_server.rs:155`, `backend/src/bin/lsp_server/server/language_server.rs:222`.
- Команда `bsl.parseConfiguration` использует server-initiated WorkDoneProgress и теперь завершает `End` **после** индексации BSL модулей: `backend/src/bin/lsp_server/commands/configuration.rs:109`, `backend/src/bin/lsp_server/commands/configuration.rs:298`, `backend/src/bin/lsp_server/commands/configuration.rs:361`.
- `bsl/serverStatus` используется только для “loading/ready” статуса (отдельно от процентов): `backend/src/bin/lsp_server/types.rs:12`, `backend/src/bin/lsp_server/server/language_server.rs:177`.

### Что уже есть в расширении
- Есть обработчик `$/progress`, который рисует прогресс в `ProgressLocation.Window`, **но состояние одно на всё** (нет map по token): `vscode-extension/src/lsp/client/progress-handler.ts:25`.
- Для некоторых команд расширение до сих пор показывает локальный `withProgress(Notification)` с условными инкрементами (дублирование UI поверх `$/progress`): `vscode-extension/src/commands/index-commands.ts:43`, `vscode-extension/src/commands/index-commands.ts:118`.
- Команда `parseConfiguration` в расширении больше не дублирует прогресс локальным `withProgress` (ориентир на `$/progress`): `vscode-extension/src/commands/parseConfiguration.ts:28`.

### Что отсутствует
- Нет детального прогресса “парсинг BSL модулей X/Y” (сейчас это один этап 90→99% на команду): `backend/src/data/loaders/config_bsl_modules.rs:70`.
- Нет протокольного/серверного API для incremental reindex (в расширении есть `bslAnalyzer.incrementalUpdate`, но на сервере нет обработчика кастомного метода `bsl/incrementalUpdate`): `vscode-extension/src/lsp/customRequests.ts:225`, `backend/src/bin/lsp_server/server/language_server.rs:688`.
- В расширении `$/progress` не различает параллельные токены (startup load vs parseConfiguration vs reindex) → возможны “перетирания” прогресса.

---

## Принципы (источник истины)

1) **Сервер — источник истины** для прогресса CPU/IO-heavy операций (парсинг docs/config, индексация BSL, buildIndex, incremental update).  
2) **Единый транспорт прогресса**: server-initiated WorkDoneProgress (`$/progress`) + опционально `bsl/serverStatus` для статуса “loading/ready”.  
3) **Проценты монотонны** (никогда не уменьшаются).  
4) **Multi-token**: несколько задач прогресса одновременно допустимы; UI должен отображать их корректно (не один глобальный state).  
5) **Throttling** по времени (100–250ms) и “last update always delivered”.

---

## Milestones (вертикальные срезы)

### P1: Multi-token progress в расширении 🟡
**Цель:** `$/progress` отображает параллельные задачи корректно.

**Подход:**
- `Map<token, ProgressState>` вместо одного `state`.
- `begin/report/end` маршрутизируются по `params.token`.

**Критерии успеха:**
- Startup progress и progress команды `parseConfiguration` не конфликтуют.
- Прогресс реиндексации (см. P5) не “гасит” прогресс загрузки типов и наоборот.

**Проверка:**
- ❌ сейчас хранится ровно один `ProgressState`: `vscode-extension/src/lsp/client/progress-handler.ts:25`.

---

### P2: Унифицированный progress bridge на сервере 🟡
**Цель:** убрать ручное форматирование/проценты из отдельных команд и сделать один “движок” прогресса.

**Подход:**
- Ввести `ProgressReporter` (trait) и `LspWorkDoneReporter` (адаптер).
- Ввести `ProgressPlan` (веса/диапазоны стадий) для `startup` / `parseConfiguration` / `buildIndex` / `incrementalUpdate`.

**Критерии успеха:**
- Внутренние loader’ы репортят `current/total/message`, проценты считаются централизованно.
- Проценты не “прыгают” и не доходят до 100% раньше фактического конца.

---

### P3: Детальный прогресс по BSL-модулям (X/Y) 🟡
**Цель:** при индексации `*.bsl` модулей показывать прогресс по файлам.

**Подход:**
- Добавить опциональный callback/reporter в `index_configuration_bsl_modules`.
- До парсинга собрать список модулей → `total`.
- Репортить раз в N файлов/по throttling: `Indexed i/total: <path>`.

**Критерии успеха:**
- В `bsl.parseConfiguration` прогресс в стадии “Indexing configuration module methods (*.bsl)” отражает i/total.

**Проверка:**
- ❌ индексация модулей сейчас без callback: `backend/src/data/loaders/config_bsl_modules.rs:70`.

---

### P4: Устранить дублирование прогресса в UI 🟡
**Цель:** не показывать два прогресса одновременно (Notification + Window) для одной операции.

**Подход:**
- Команды, которые инициируют серверные операции (buildIndex/incrementalUpdate), не рисуют локальный `withProgress` с условными инкрементами.
- Использовать `$/progress` как основной UI, а Notification оставить только для итогового сообщения.

**Проверка:**
- ✅ `parseConfiguration` уже так сделан: `vscode-extension/src/commands/parseConfiguration.ts:28`.
- ❌ `buildIndex` и `incrementalUpdate` до сих пор рисуют локальный прогресс: `vscode-extension/src/commands/index-commands.ts:43`, `vscode-extension/src/commands/index-commands.ts:118`.

---

### P5: Реиндексация “на лету” (после правок конфигурации) 🟡
**Цель:** когда конфигурация/модули меняются на диске (выгрузка новой версии, пересборка, обновление расширения конфы), индекс обновляется без перезапуска, с понятным прогрессом.

**Сценарии:**
1) **Ручной инкрементальный апдейт** (команда): `bslAnalyzer.incrementalUpdate`.
2) **Авто-реиндексация**: при изменении файлов конфигурации (например `Configuration.xml`, `*/Ext/*.bsl`, `Forms/*/Form.xml`) расширение/сервер инициирует incremental update.

**Архитектурный выбор (рекомендуемо):**
- Сервер получает новый кастомный `workspace/executeCommand` (или custom request) `bsl.incrementalUpdate` и внутри запускает пайплайн:
  - discovery diff → перечень затронутых объектов/модулей
  - обновление `TypeRepository`/`SignatureIndex` частями
  - инвалидирование кэшей/IR по затронутым файлам
- Прогресс идёт через отдельный WorkDoneProgress token `bsl-incremental-update-*`.

**Критерии успеха:**
- Команда `bslAnalyzer.incrementalUpdate` показывает прогресс через `$/progress` (а не локальный “условный”).
- Можно “переподхватить” изменения конфигурации без restart LSP.
- Прогресс отражает стадии (discover changes / reload types / reindex bsl modules / finalize).

**Проверка:**
- ✅ команда в расширении есть: `vscode-extension/src/commands/index-commands.ts:111`.
- ❌ сервер не реализует `bsl/incrementalUpdate` (сейчас executeCommand поддерживает только `bsl.*` commands): `backend/src/bin/lsp_server/server/language_server.rs:688`.

---

### P6: Тесты прогресса (unit/integration/e2e) 🟡
**Цель:** прогресс не ломается при рефакторинге и поддерживает multi-token.

**Критерии успеха:**
- Unit: расчёт процентов и монотонность (серверный progress engine).
- Integration: `$/progress` handler корректно маршрутизирует по token.
- E2E: smoke-тест на progress begin/report/end для startup + parseConfiguration + incrementalUpdate.

---

## Следующие шаги (минимальный путь)

1) P1: переписать `$/progress` handler на `Map<token, state>` (без изменения UX).
2) P5 (часть 1): реализовать серверный endpoint для `incrementalUpdate` и прогресс токен.
3) P4: убрать локальный `withProgress` из `buildIndex/incrementalUpdate` команд (оставить итоговый toast).
4) P3: добавить детальный прогресс по `*.bsl` модулям.
5) P2: централизовать расчёт процентов (убрать магические числа из команд).

