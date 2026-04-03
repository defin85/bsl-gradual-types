# bsl-intellisense Specification

## Purpose
TBD - created by archiving change audit-bsl-intellisense. Update Purpose after archive.
## Requirements
### Requirement: LSP‑сервер предоставляет базовые функции IntelliSense для BSL
Система SHALL предоставлять LSP‑сервер для BSL, который поддерживает базовые функции IntelliSense:
- `textDocument/completion` и `completionItem/resolve`,
- `textDocument/hover`,
- `textDocument/signatureHelp`,
- `textDocument/definition` (как минимум для навигации по определениям типов),
- публикацию диагностик через `textDocument/publishDiagnostics`.

#### Scenario: IDE получает подсказки и диагностику через LSP
- **GIVEN** LSP‑сервер запущен и рабочая область содержит `.bsl` файл
- **WHEN** клиент запрашивает completion/hover/signatureHelp/definition и получает diagnostics при изменении текста
- **THEN** сервер возвращает корректные ответы по протоколу LSP и публикует diagnostics для текущей версии документа

### Requirement: VS Code extension запускает LSP и предоставляет IDE‑интеграцию
Система SHALL предоставлять VS Code extension, который запускает LSP-сервер, прокидывает настройки (пути к документации платформы/конфигурации) и обеспечивает базовую IDE-интеграцию для BSL.

В части completion extension MUST:
- не блокировать и не подменять trigger-character completion pipeline LSP для BSL;
- обеспечивать предсказуемую диагностику клиентской конфигурации, когда effective editor settings отключают автотриггер suggestions по trigger symbols.

#### Scenario: Расширение VS Code поднимает LSP и включает базовые подсказки
- **GIVEN** пользователь открыл workspace с `.bsl` файлами в VS Code
- **WHEN** расширение активируется
- **THEN** LSP-клиент стартует, и пользователь получает completion/hover/signatureHelp/diagnostics в редакторе

#### Scenario: Автотриггер completion по `.` отключён в effective settings
- **GIVEN** для BSL effective конфигурация редактора отключает trigger-based suggestions (например, `editor.suggestOnTriggerCharacters=false`)
- **WHEN** расширение инициализирует LSP-интеграцию
- **THEN** extension явно логирует предупреждение с причиной и шагом исправления
- **AND** extension не меняет пользовательские editor settings автоматически

### Requirement: Стратегия форматирования BSL документирована до реализации
Система SHALL документировать выбранную стратегию форматирования BSL (цели, non-goals, ограничения и способ интеграции в IDE) до добавления LSP formatting.

#### Scenario: Команда согласовала форматирование до включения в IDE
- **GIVEN** проект планирует добавить форматирование в IDE
- **WHEN** change по форматированию проходит ревью
- **THEN** стратегия форматирования (и причины выбора) зафиксированы и понятны поддерживающим

### Requirement: LSP поддерживает форматирование BSL в IDE (SHOULD)
Система SHALL поддерживать форматирование BSL в IDE через LSP:
- `textDocument/formatting`,
- (опционально) `textDocument/rangeFormatting`,
при условии, что стратегия форматтера выбрана и документирована.

Поддержка форматирования SHALL быть конфигурируемой (включаемой/выключаемой).

#### Scenario: Пользователь форматирует документ
- **GIVEN** форматирование включено и стратегия форматтера определена
- **WHEN** IDE запрашивает `textDocument/formatting` для `.bsl` документа
- **THEN** сервер возвращает детерминированный набор правок с минимальным diff

#### Scenario: Форматирование можно отключить
- **GIVEN** форматирование отключено настройкой
- **WHEN** IDE запрашивает `textDocument/formatting`
- **THEN** сервер не заявляет поддержку formatting в capabilities либо возвращает предсказуемый отказ (и не создаёт ложных ожиданий)

### Requirement: LSP поддерживает структуру документа через documentSymbol
Система SHALL:
- объявлять `document_symbol_provider` в `ServerCapabilities`,
- реализовать `textDocument/documentSymbol` для `.bsl` документов.

Минимальная модель символов SHOULD включать:
- процедуры/функции,
- группировку по областям (`#Область`/`#КонецОбласти`) как вложенность.

Сервер MUST возвращать корректные ranges в терминах LSP (UTF-16 позиции) и детерминированный результат для одинакового текста.

#### Scenario: IDE строит outline по documentSymbol
- **GIVEN** открыт `.bsl` файл с процедурами/функциями и областями
- **WHEN** клиент вызывает `textDocument/documentSymbol`
- **THEN** сервер возвращает иерархию символов, пригодную для Outline

### Requirement: LSP поддерживает поиск символов workspace через workspace/symbol (MVP)
Система SHALL:
- объявлять `workspace_symbol_provider` в `ServerCapabilities`,
- реализовать `workspace/symbol`.

MVP-ограничение: поиск SHOULD выполняться по документам, доступным серверу в текущей сессии (минимально: открытые документы).

Сервер MUST:
- возвращать детерминированный порядок элементов,
- ограничивать размер ответа.

#### Scenario: Go to Symbol in Workspace находит символ в открытых документах
- **GIVEN** в сессии открыто несколько `.bsl` файлов с процедурами/функциями
- **WHEN** клиент вызывает `workspace/symbol` с query
- **THEN** сервер возвращает элементы, удовлетворяющие запросу, с корректными `Location` и `uri`

### Requirement: LSP поддерживает поиск ссылок и rename для поддерживаемых символов
Система SHALL поддерживать:
- `textDocument/references` для поиска ссылок на символ,
- `textDocument/rename` (и SHOULD `prepareRename`) для безопасного переименования поддерживаемых символов.

Ограничения (какие символы поддерживаются) MUST быть задокументированы и неизменно применяться сервером.

MVP поддерживаемые символы:
- локальные переменные, объявленные через `VarDeclaration` внутри процедуры/функции (rename/references в пределах одной процедуры/функции),
- процедуры/функции, объявленные в документе, и их прямые вызовы вида `Identifier(...)` в этом же документе.

Не поддерживаются (MVP):
- динамические обращения (строковые имена, косвенные вызовы, рефлексия),
- вызовы через property access (`Obj.Method()`), если под курсором не сам declaration.

#### Scenario: Find References возвращает ссылки на локальную переменную
- **GIVEN** в документе есть локальная переменная и её использования
- **WHEN** клиент вызывает `textDocument/references`
- **THEN** сервер возвращает locations всех ссылок (с учётом includeDeclaration)

#### Scenario: Rename меняет все ссылки на символ и не трогает другие имена
- **GIVEN** в документе есть символ и несколько ссылок на него
- **WHEN** клиент вызывает `textDocument/rename`
- **THEN** сервер возвращает WorkspaceEdit, который меняет только ссылки на этот символ

### Requirement: VS Code extension не регистрирует заглушки IntelliSense по умолчанию
Система SHALL обеспечивать, что VS Code extension не регистрирует “пустые” (stub) IntelliSense providers по умолчанию. Если provider зарегистрирован, он MUST возвращать осмысленный результат.

#### Scenario: Inlay hints / code actions не являются заглушками
- **GIVEN** пользователь включил `bsl.typeHints.enabled` и/или `bsl.codeActions.enabled`
- **WHEN** IDE запрашивает inlay hints или code actions
- **THEN** extension использует стандартный LSP pipeline (без кастомных заглушек), и фичи появляются только если сервер объявил соответствующие capabilities
- **AND** если сервер не объявил capability, extension явно логирует предупреждение и не обещает фичу пользователю

### Requirement: LSP поддерживает inlay hints по типам (конфигурируемо)
Система SHALL поддерживать `textDocument/inlayHint` для `.bsl/.os` документов, чтобы IDE могла показывать подсказки типов (type hints) в коде.

Поддержка inlay hints MUST:
- быть конфигурируемой (включаемой/выключаемой),
- быть детерминированной (одинаковый текст → одинаковый результат),
- иметь лимит на размер ответа.

MVP-границы:
- сервер генерирует подсказки типов для локальных переменных как `: <TypeName>` в местах присваивания (идентификатор слева в `X = ...;`).
- (опционально в будущем) подсказки return type для функций.

#### Scenario: Inlay hints включены и возвращают осмысленные результаты
- **GIVEN** включены type hints (feature gate) и настроены пороги шумности
- **WHEN** IDE вызывает `textDocument/inlayHint` для диапазона в документе
- **THEN** сервер возвращает hints типа `: <TypeName>` в релевантных местах (минимум: локальные переменные)
- **AND** результат детерминирован и ограничен по размеру

#### Scenario: Inlay hints выключены
- **GIVEN** type hints выключены настройкой
- **WHEN** IDE пытается использовать hints
- **THEN** сервер не заявляет `inlayHintProvider` в capabilities либо возвращает предсказуемый отказ (без ложных ожиданий)

### Requirement: LSP поддерживает code actions (MVP) без заглушек
Система SHALL поддерживать `textDocument/codeAction` для предоставления пользователю quick fixes и/или простых refactors, при этом:
- сервер SHALL не заявлять `codeActionProvider`, если не способен вернуть осмысленные результаты,
- поддерживаемое множество действий MUST быть задокументировано (MVP-границы).

MVP-границы (документировано):
- QuickFix: `Add type annotation for '<Var>'`
  - применимо, когда в документе есть `Перем <Var>;` без `: Тип`
  - и сервер может вывести тип `<Var>` (детерминированно) по данным анализа, без regex по тексту diagnostics
  - правка: вставка `: <TypeName>` в объявление `Перем`.
- RefactorExtract: `Extract to variable 'tmp'`
  - применимо только для непустого выделения в пределах одной строки
  - правка: вставка строки `tmp = <expr>;` перед текущей строкой + замена выделения на `tmp`.

#### Scenario: IDE показывает code actions, когда они применимы
- **GIVEN** code actions включены (feature gate)
- **WHEN** IDE запрашивает `textDocument/codeAction` для диапазона в документе
- **THEN** сервер возвращает применимые code actions (минимум: один refactor и один quick fix в пределах документа)
- **AND** server не возвращает “пустые заглушки” вместо действий

#### Scenario: Code actions выключены
- **GIVEN** code actions выключены
- **WHEN** IDE запрашивает code actions
- **THEN** сервер не заявляет `codeActionProvider` в capabilities либо возвращает предсказуемый отказ

### Requirement: VS Code sidebar использует один activity bar контейнер для BSL Analyzer (MUST)
Система MUST регистрировать sidebar расширения в одном activity bar container для BSL Analyzer.

`Overview`, `Diagnostics`, `Type Repository`, `Quick Actions` и `Cache Dashboard` MUST быть доступны внутри этого единого container.

#### Scenario: Пользователь видит один вход в sidebar расширения
- **GIVEN** расширение активировано в VS Code
- **WHEN** пользователь открывает Activity Bar
- **THEN** отображается один container BSL Analyzer
- **AND** внутри него доступны разделы overview/diagnostics/type repository/quick actions/cache dashboard

### Requirement: Счётчики типов консистентны между sidebar виджетами (MUST)
Система MUST формировать счётчики `TypeRepository` (`total`, `platform`, `configuration`) из единого snapshot/revision источника.

`Overview`, `Type Repository` и `Quick Actions` MUST отображать согласованные значения для одного и того же snapshot состояния.

#### Scenario: Platform count совпадает в Overview, Type Repository и Quick Actions
- **GIVEN** sidebar обновлён на одном snapshot type repository
- **WHEN** пользователь сравнивает значения в `Overview`, `Type Repository` и `Quick Actions`
- **THEN** platform/config/total counts не противоречат друг другу

### Requirement: Summary diagnostics в sidebar согласован с фактическим списком diagnostics (MUST)
Система MUST обеспечивать, что summary (`Issues Found`) и содержимое раздела `Diagnostics` рассчитываются из согласованного источника данных в рамках одного snapshot.

Система MUST NOT показывать одновременно "No issues found" и ненулевой summary issues для одного и того же состояния.

#### Scenario: Summary и diagnostics tree не противоречат
- **GIVEN** workspace snapshot содержит N диагностик
- **WHEN** пользователь смотрит `Overview` и `Diagnostics`
- **THEN** summary отражает те же diagnostics, что и дерево по severity
- **AND** не возникает конфликтующих статусов "issues > 0" и "No issues found"

### Requirement: Quick Actions использует live-метрики вместо статических значений (MUST)
Система MUST получать отображаемые счётчики в Quick Actions из live-данных LSP/TypeRepository и MUST NOT использовать хардкодные числовые значения.

#### Scenario: Счётчик типов в Quick Actions обновляется после изменения индекса
- **GIVEN** количество platform types изменилось после переиндексации
- **WHEN** пользователь открывает/обновляет Quick Actions
- **THEN** отображается актуальное значение из live-метрик, а не фиксированный хардкод

### Requirement: User-facing sidebar UI не показывает сырые internal tokens (MUST)
Система MUST отображать статусы и иконки в sidebar через корректные UI-примитивы VS Code и MUST NOT показывать неотрендеренные токены формата `$(...)` в пользовательских строках.

#### Scenario: Статус сервера отображается без сырых токенов
- **GIVEN** статус LSP сервера равен Running
- **WHEN** пользователь открывает раздел `LSP Server Status`
- **THEN** статус отображается как корректный UI-текст/иконка
- **AND** строка не содержит сырых фрагментов вроде `$(check)`

### Requirement: Startup full-index выполняется без дублирования между LSP startup и `bsl/buildIndex` (MUST)
Система MUST обеспечивать single-flight поведение для full-index операций при старте IDE:
- если LSP startup full-index уже выполняется, дополнительный запрос `bsl/buildIndex` MUST NOT запускать второй full-index процесс;
- если full-index уже завершён и состояние `ready`, extension startup MUST NOT инициировать лишний full-index.

#### Scenario: Запрос `bsl/buildIndex` во время startup не запускает второй full-index
- **GIVEN** LSP находится в состоянии startup full-index (`running`)
- **WHEN** extension (или пользователь) вызывает `bsl/buildIndex`
- **THEN** сервер не запускает второй full-index процесс
- **AND** возвращает детерминированный attach-статус (`already running`) с идентификатором текущей операции

#### Scenario: После успешного startup extension не запускает повторный full-index
- **GIVEN** LSP сообщает состояние индекса `ready=true`
- **WHEN** extension завершает активацию
- **THEN** extension не инициирует дополнительный full-index на старте

### Requirement: LSP предоставляет machine-readable контракт состояния индекса `bsl/getIndexState` (MUST)
LSP MUST предоставлять custom request `bsl/getIndexState` с contract version `1`, включающим:
- `version`,
- `state` (`idle|running|ready|failed`),
- `ready`,
- `active_operation`,
- `operation_id`,
- `message`,
- `updated_at_ms`.

Поля `active_operation`, `operation_id`, `message` MUST присутствовать в ответе всегда; при отсутствии значения сервер MUST возвращать `null` (а не пропускать поле).

Клиент MUST использовать этот контракт как источник истины для startup orchestration full-index.

#### Scenario: Клиент получает `running` состояние активной операции
- **GIVEN** сервер выполняет startup full-index
- **WHEN** extension вызывает `bsl/getIndexState`
- **THEN** сервер возвращает `state=running`
- **AND** указывает `active_operation` и `operation_id`

#### Scenario: Клиент получает `idle` с явными nullable полями
- **GIVEN** full-index не выполняется и состояние сервера `idle`
- **WHEN** extension вызывает `bsl/getIndexState`
- **THEN** сервер возвращает `active_operation=null`, `operation_id=null`, `message=null`
- **AND** поля присутствуют в payload явно

### Requirement: Startup orchestration индекса в extension опирается на server-driven index state (MUST)
VS Code extension MUST принимать решение о запуске full-index на старте по machine-readable состоянию индекса, предоставленному LSP.

Extension MUST NOT использовать локальный filesystem sentinel (`project_indices/.../unified_index.json`) как единственный источник истины для startup решения о full-index.

#### Scenario: Локальный sentinel отсутствует, но сервер уже готов
- **GIVEN** локальный файл sentinel отсутствует или устарел
- **AND** LSP возвращает `ready=true` для index state
- **WHEN** extension выполняет startup orchestration
- **THEN** full-index не запускается повторно только из-за отсутствия sentinel

#### Scenario: Сервер сообщает `failed/idle`, и auto-index включён
- **GIVEN** LSP возвращает `state=failed` или `state=idle`
- **AND** настройка auto-index в extension включена
- **WHEN** extension завершает активацию
- **THEN** extension инициирует один full-index запуск через `bsl/buildIndex`
- **AND** при повторном запросе во время выполнения соблюдается single-flight поведение

#### Scenario: Legacy LSP не поддерживает `bsl/getIndexState`
- **GIVEN** extension подключён к LSP версии без `bsl/getIndexState` (ответ `Method not found`)
- **WHEN** extension выполняет startup orchestration
- **THEN** extension не запускает silent full-index автоматически (fail-closed)
- **AND** показывает явное предупреждение о несовместимости
- **AND** оставляет доступной ручную команду `Build Index`

### Requirement: Running-состояние full-index имеет fail-safe timeout (MUST)
Система MUST иметь watchdog timeout для full-index в состоянии `running`.

При превышении timeout система MUST переводить состояние в `failed` и очищать признак активной операции, чтобы последующий retry мог быть выполнен детерминированно.

#### Scenario: Зависшая операция выходит в `failed` по timeout
- **GIVEN** full-index находится в `running` дольше configured timeout
- **WHEN** watchdog фиксирует превышение лимита
- **THEN** состояние индекса переводится в `failed`
- **AND** `active_operation`/`operation_id` сбрасываются
- **AND** следующий ручной или startup-triggered build может запуститься заново

### Requirement: VS Code extension показывает per-request completion timeline в панели Observability (MUST)
VS Code extension MUST предоставлять user-facing completion observability UI в контейнере `bslAnalyzer`.

Observability completion UI MUST:
- читать authoritative server trace только из server-driven LSP контракта `bsl.getCompletionTimeline`;
- читать server trace через request path `workspace/executeCommand` (`command: bsl.getCompletionTimeline`) как единственный transport для server-side части этой capability;
- быть реализован как `webview` view (`WebviewViewProvider`) внутри контейнера `bslAnalyzer`;
- содержать отдельный `Server Timeline` section;
- содержать отдельный local-only `Client Probe Feed` section;
- показывать total duration, outcome и список stage entries для выбранного server trace;
- визуально выделять dominant stage (самый длительный server этап);
- отображать статус каждого server этапа (`completed|cancelled|failed|skipped`);
- явно маркировать `Client Probe Feed` как local-only debug data, не эквивалентные server timeline;
- отображать в `Client Probe Feed`, когда они доступны, bounded cancellation diagnostics, transport-phase diagnostics, result-shape diagnostics и version-drift/overlap diagnostics;
- отображать в `Server Timeline`, когда они доступны, bounded server-edge transport/cancellation diagnostics из authoritative server trace.

Observability completion UI MUST NOT:
- реконструировать per-request server timeline из текстовых логов или агрегированных p50/p95/p99 метрик;
- использовать `TreeDataProvider` как реализацию timeline capability;
- подставлять отсутствующие server stages, routes или outcomes из client-side probe;
- скрывать server trace только потому, что local probes отсутствуют;
- выполнять trace-level correlation между server trace и local probes в рамках этого change;
- подставлять server-edge diagnostics из client-side probe, если серверный payload их не содержит.

#### Scenario: Пользователь отличает queue-before-handler от долгого server execution
- **GIVEN** authoritative `Server Timeline` trace содержит `server_edge_details`
- **WHEN** пользователь открывает completion observability UI
- **THEN** panel показывает bounded server-edge diagnostics для `transport_to_handler_wait` и `server_handler_exec`
- **AND** эти diagnostics остаются частью `Server Timeline`, а не `Client Probe Feed`

#### Scenario: Legacy timeline payload без server-edge diagnostics остаётся читаемым
- **GIVEN** connected server возвращает payload `version=2` без `server_edge_details`
- **WHEN** пользователь открывает completion observability UI
- **THEN** extension продолжает показывать `Server Timeline` и `Client Probe Feed`
- **AND** не пытается выдумывать отсутствующие server-edge diagnostics
- **AND** отсутствие новых полей не ломает rendering/copy flow

### Requirement: Timeline panel деградирует предсказуемо с legacy LSP (MUST)
Если подключённый LSP не поддерживает `bsl.getCompletionTimeline`, extension MUST fail-closed для server-side timeline capability:
- показывать явный user-facing статус несовместимости для `Server Timeline`;
- не падать и не ломать остальные разделы Observability/Sidebar;
- не маскировать отсутствие authoritative server timeline local probes-данными.

При этом `Client Probe Feed` MAY оставаться доступным как local-only debug stream, если probes уже записываются в extension.

#### Scenario: Legacy LSP не поддерживает server timeline request
- **GIVEN** `bsl.getCompletionTimeline` возвращает `Method not found`
- **WHEN** пользователь открывает completion observability UI
- **THEN** extension показывает понятное сообщение о неподдерживаемой версии сервера для `Server Timeline`
- **AND** оставляет рабочими другие observability views и команды
- **AND** если `Client Probe Feed` доступен, он явно помечен как local-only и не заменяет server timeline

### Requirement: VS Code extension ведёт bounded client-side completion probe buffer (MUST)
VS Code extension MUST вести bounded in-memory ring buffer последних client-side completion probes на основном activation/runtime path.

Probe buffer MUST:

- быть wired на default `LanguageClient` path, используемый обычной активацией extension;
- использовать deterministic oldest-first eviction;
- хранить только bounded/redacted probe fields;
- оставаться session-local и in-memory only.

Каждый probe MUST включать только bounded metadata:

- `probe_id`;
- `uri`;
- `document_version`;
- `document_version_at_terminal`;
- `trigger_mode` и optional `trigger_character`;
- `request_started_at_ms`;
- `request_completed_at_ms`;
- explicit transport-phase milestones, достаточные для отделения:
  - client enter;
  - LSP request dispatch;
  - raw transport response receive;
  - LSP promise resolve;
  - client terminal;
- terminal status/result summary;
- bounded `result_kind` vocabulary;
- bounded `item_count_bucket`;
- `is_incomplete`, только если этот сигнал доступен без guesswork;
- `time_since_last_local_edit_ms`;
- `time_since_last_did_change_sent_ms` либо явное значение `unknown`, если этот сигнал недоступен;
- bounded cancellation diagnostics: `cancel_reason_hint` из vocabulary `superseded_same_version|superseded_newer_version|editor_state_changed|unknown`, optional `superseded_by_probe_id`, optional `superseded_after_ms`;
- bounded overlap/drift diagnostics: `did_change_count_during_probe`, `cursor_moved_during_probe`, `active_completion_count_at_start`, `same_uri_probe_overlap_count`, `newer_probe_started_before_terminal`;
- derived context flags вроде `is_after_dot` и `identifier_tail_length`.

Если raw transport response receive boundary недоступна на конкретном runtime path, probe MUST фиксировать это explicit bounded marker'ом unavailable/unknown и MUST NOT silently подменять receive timestamp временем promise resolution.

Probe buffer MUST NOT:

- хранить raw document text, line prefixes или произвольные snippets;
- хранить unbounded free-form labels;
- требовать отдельного persistent telemetry pipeline в рамках этой capability;
- требовать protocol-level `client_probe_id` или trace-level correlation с `Server Timeline`.

#### Scenario: Probe отделяет raw receive от promise resolution

- **GIVEN** completion probe завершился успешным LSP response
- **WHEN** extension записывает transport-phase milestones
- **THEN** probe отдельно фиксирует raw transport response receive и LSP promise resolve
- **AND** не смешивает эти две границы в один timestamp

#### Scenario: Недоступный receive boundary не подменяется guessed timestamp

- **GIVEN** на конкретном runtime seam raw transport receive boundary нельзя наблюдать детерминированно
- **WHEN** extension завершает запись client-side probe
- **THEN** probe явно помечает receive boundary как unavailable или unknown
- **AND** не записывает promise-resolution timestamp под видом raw receive

### Requirement: Existing completion surfaces переносят `v9` pre-service-scope split без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v11` service-future first-poll / first-wake split в человекочитаемом виде.

Human-readable projection MUST:
- показывать first-poll / first-wake split, если connected server возвращает `v11` payload;
- сохранять уже существующие `v10` dispatch split, `v9` pre-service-scope split и truthful provenance rules;
- явно деградировать на `v10`, не выдумывая `service_future_first_poll_entered_at_ms`, `service_future_first_poll_outcome`, `service_future_first_wake_scheduled_at_ms` и `first_poll_to_first_wake_wait_ms`;
- для incident bundle не скрывать эту limitation за нейтральным `No gaps were recorded`.

#### Scenario: Panel и clipboard показывают first-poll / first-wake split рядом с existing ingress facts
- **GIVEN** extension получает completion timeline `v11` с bounded first-poll facts
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый split рядом с existing dispatch, pre-service-scope и pre-method fields
- **AND** оператор может отличить lag до первого poll future от lag после первого `Pending`

#### Scenario: Incident bundle summary переносит `v11` split без guessed reconstruction
- **GIVEN** incident bundle строится по `v11` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts для first-poll / first-wake split
- **AND** derived handoff не выдумывает этот split для `v10` payload

#### Scenario: Extension явно деградирует на `v10`
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v11` fields
- **AND** человекочитаемый output явно отмечает, что first poll / wake split unavailable by design

#### Scenario: Incident bundle не маскирует отсутствие `v11` split как отсутствие gaps
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension формирует `summary.md` для incident bundle
- **THEN** summary явно отмечает, что first poll / wake split unavailable by design for `contract=v10`
- **AND** summary не должен одновременно утверждать, что для этого missing split `No gaps were recorded`

### Requirement: Existing completion surfaces различают strong и weak pre-method attribution без invented findings (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v8` pre-method attribution provenance в человекочитаемом виде.

Human-readable projection MUST:
- явно показывать provenance для pre-method attribution, если connected server возвращает `v8` payload;
- считать `server_before_method_entry_dominant` сильным verdict только для same-request authoritative provenance;
- явно деградировать на `v7`, не выдумывая provenance для старого payload.

#### Scenario: Panel и clipboard показывают provenance рядом с pre-method split
- **GIVEN** extension получает completion timeline `v8` с pre-method provenance
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает pre-method split вместе с provenance
- **AND** оператор может отличить strong same-request attribution от best-effort fallback

#### Scenario: Incident bundle findings не агрегируют weak attribution как сильный ingress bottleneck
- **GIVEN** incident bundle строится по `v8` payload, где trace использует best-effort fallback provenance
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw fact
- **AND** derived findings не считают такой trace сильным `server_before_method_entry` bottleneck

#### Scenario: Extension явно деградирует на `v7`
- **GIVEN** connected server возвращает completion timeline `v7`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `v8` provenance
- **AND** человекочитаемый output явно отмечает, что trustworthy pre-method attribution fields unavailable by design

### Requirement: Existing completion surfaces переносят `v7` pre-method и snapshot overshoot facts без invented data (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить новые `v7` authoritative facts в человекочитаемом виде и MUST явно деградировать на `v6`, не реконструируя отсутствующие поля эвристикой.

Human-readable projection MUST:
- показывать pre-method split отдельно от уже существующих `transport_to_method_wait_ms` / `transport_to_handler_wait_ms`;
- показывать bounded `snapshot_with_deps_timeout_runtime`, если он доступен;
- явно указывать, что `v7` fields unavailable by design, если bundle построен по `v6` payload.

#### Scenario: Panel и clipboard показывают новый pre-method split
- **GIVEN** extension получает authoritative completion timeline `v7` с bounded pre-method split
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает отдельные fact lines для pre-method split
- **AND** оператору не нужно открывать raw JSON, чтобы увидеть этот split

#### Scenario: Incident bundle summary показывает snapshot overshoot attribution
- **GIVEN** incident bundle построен по `v7` payload, где `prepare_timeout` содержит `snapshot_with_deps_timeout_runtime`
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request-centric summary переносит этот bounded fact в derived handoff
- **AND** summary не заменяет его guessed причиной

#### Scenario: Extension явно деградирует на `v6`
- **GIVEN** connected server возвращает completion timeline `v6`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `service_scope_*` или `snapshot_with_deps_timeout_runtime`
- **AND** человекочитаемый output явно отмечает отсутствие `v7` attribution fields

### Requirement: Incident bundle findings агрегируют ingress verdicts truthfully (MUST)
`incident.json` и `summary.md` MUST агрегировать ingress-related findings только из truthful positive-only verdicts и MUST NOT переоценивать ingress bottleneck на hot traces, где положительный ingress wait отсутствует.

Request-centric bundle summary MUST:
- использовать тот же смысл ingress verdicts, что и другие completion projections extension;
- считать client-side и server-side ingress отдельно, если соответствующие verdicts доступны;
- не формулировать общий ingress bottleneck только на основании traces с нулевыми ingress waits;
- сохранять request summary валидным, даже если client correlation unavailable.

#### Scenario: Summary не переоценивает hot traces как ingress bottleneck
- **GIVEN** capture window содержит hot completion trace с нулевыми ingress/prelude waits
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** этот trace не учитывается в ingress findings
- **AND** summary не заявляет ingress bottleneck для него

#### Scenario: Summary различает client-side и server-side ingress
- **GIVEN** capture window содержит как минимум один correlated trace с доминирующим `client_to_transport_wait_ms`
- **AND** содержит trace с доминирующим `transport_to_method_wait_ms`
- **WHEN** extension формирует derived request-centric summary
- **THEN** findings и request entries различают client-side и server-side ingress verdicts
- **AND** оператору не нужно открывать raw JSON, чтобы увидеть этот split

#### Scenario: Correlation gap не превращается в guessed ingress finding
- **GIVEN** request summary не имеет deterministic client correlation
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary не создаёт client-side ingress finding для такого request
- **AND** request остаётся server-centric или без ingress finding, если положительный server-side ingress wait отсутствует

### Requirement: Observability incident bundle даёт request-centric handoff summary поверх raw evidence (MUST)
VS Code extension MUST формировать `incident.json` и `summary.md` так, чтобы типовой completion incident можно было разбирать как набор bounded request-level facts без обязательного чтения полного raw timeline JSON.

Этот derived report MUST:
- сохранять raw attachments отдельными и не подменять их;
- использовать authoritative request list только из `bsl.getCompletionTimeline`, если этот источник доступен;
- выражать capture scope (`uri` или явное отсутствие single-URI scope) без guesswork;
- выражать `request_count`;
- содержать bounded request list для authoritative completion traces;
- переносить в request list ключевые latency/verdict facts из authoritative trace;
- использовать client probes только как optional supplemental correlation layer;
- явно маркировать unavailable/unsupported/ambiguous correlation;
- не вычислять псевдо-`metrics delta` из одного cumulative snapshot.

#### Scenario: Single-document capture получает request-centric summary
- **GIVEN** export bundle содержит authoritative completion timeline, и все captured traces относятся к одному `uri`
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** derived report явно содержит этот `uri`
- **AND** derived report содержит `request_count`
- **AND** derived report включает bounded request list с ключевыми latency/verdict facts для каждого authoritative trace

#### Scenario: Derived report не подменяет missing authoritative request list local probes-данными
- **GIVEN** connected server не вернул authoritative completion timeline
- **WHEN** extension формирует bundle
- **THEN** bundle остаётся валидным partial export
- **AND** request-centric section явно помечается как unavailable или unsupported
- **AND** local client probes не выдаются за authoritative request list

### Requirement: Probe-to-trace correlation остаётся deterministic и fail-closed (MUST)
Если extension дополняет request-centric summary данными из local client probes, такая correlation MUST выполняться только по deterministic bounded rules.

Correlation MUST:
- использовать только уже записанные bounded fields из authoritative trace и probe;
- быть optional;
- не требовать нового server-side request или explicit shared request id;
- не создавать guessed pair, если correlation ambiguous.

При успешной correlation request summary MAY включать bounded client-side supplement, например:
- `probe_id`;
- `client_duration_ms`;
- `client_terminal_state`;
- optional client/server edge delta.

При ambiguous или unavailable correlation derived report MUST:
- оставить request summary валидным и server-centric;
- явно указать ограничение;
- не выдумывать client-side pair.

#### Scenario: Unambiguous correlation переносит bounded client-side supplement
- **GIVEN** authoritative trace и local probe можно сопоставить детерминированно
- **WHEN** extension строит request-centric summary
- **THEN** request entry MAY включать bounded client-side supplement
- **AND** supplement не подменяет authoritative server verdicts и latencies

#### Scenario: Ambiguous correlation не создаёт guessed pair
- **GIVEN** для authoritative trace существует несколько одинаково правдоподобных probe-кандидатов или недостаточно данных для уверенного сопоставления
- **WHEN** extension строит request-centric summary
- **THEN** request entry остаётся без client-side pair
- **AND** derived report явно фиксирует correlation gap
- **AND** bundle не создаёт guessed correlation

### Requirement: VS Code extension экспортирует AI-friendly observability incident bundle (MUST)
VS Code extension MUST предоставлять явный user-facing export surface для observability incident handoff в формате bundle, пригодном для внешнего AI/incident анализа.

Этот export bundle MUST:
- собираться extension-side поверх уже существующих observability surface-ов;
- использовать authoritative server timeline только из `bsl.getCompletionTimeline`;
- использовать observability metrics snapshot только из `bsl.getObservabilityMetrics`;
- использовать local client probes только из session-local probe buffer extension;
- включать `summary.md` как краткий human-readable report;
- включать `incident.json` как machine-readable derived report;
- включать raw attachments отдельно от derived report;
- не использовать Output panel dump text как canonical raw source;
- не требовать нового server-side custom request в первой итерации;
- не подменять существующие raw panels и copy/debug flows.

Export bundle MUST явно различать:
- authoritative server trace;
- local-only client probes;
- cumulative metrics snapshot.

#### Scenario: Пользователь экспортирует bundle из observability surface
- **GIVEN** extension подключена к LSP и observability surfaces доступны
- **WHEN** пользователь запускает export incident bundle
- **THEN** extension создаёт bundle с `summary.md` и `incident.json`
- **AND** bundle содержит raw attachments для server timeline, client probes и metrics snapshot
- **AND** summary/report не требуют ручного склеивания текста из нескольких UI панелей

#### Scenario: Raw evidence остаётся отдельным от derived summary
- **GIVEN** extension экспортирует incident bundle
- **WHEN** пользователь или внешний инструмент читает bundle
- **THEN** raw данные completion timeline, client probes и metrics snapshot доступны как отдельные attachments
- **AND** derived summary не подменяет и не перезаписывает raw evidence
- **AND** raw attachments не зависят от truncated Output formatting

### Requirement: Incident bundle деградирует предсказуемо при частичной недоступности данных (MUST)
Export incident bundle MUST завершаться fail-closed по отсутствующим sections, но fail-open для самого handoff flow: bundle может быть частичным, если некоторые источники недоступны, однако он MUST явно фиксировать gaps и MUST NOT выдумывать отсутствующие данные.

Partial export semantics MUST:
- сохранять capture metadata даже при частичной недоступности;
- явно помечать unavailable/unsupported sections в `incident.json` и `summary.md`;
- не реконструировать server trace из client probes или aggregate metrics;
- не подменять missing metrics snapshot последним текстовым dump из Output;
- оставлять raw attachments только для реально полученных sections.

#### Scenario: Legacy LSP не поддерживает `bsl.getCompletionTimeline`
- **GIVEN** connected server не поддерживает `bsl.getCompletionTimeline`
- **WHEN** пользователь запускает export incident bundle
- **THEN** export всё равно создаёт bundle
- **AND** bundle явно помечает server timeline как `unsupported`
- **AND** не пытается реконструировать authoritative server trace из local probes

#### Scenario: Metrics snapshot временно недоступен
- **GIVEN** `bsl.getObservabilityMetrics` временно недоступен или завершился ошибкой
- **WHEN** пользователь запускает export incident bundle
- **THEN** export всё равно создаёт bundle с доступными server timeline и/или client probes
- **AND** `incident.json` и `summary.md` явно фиксируют отсутствие metrics snapshot
- **AND** export не подменяет missing metrics текстом из прошлых Output dumps

### Requirement: Existing completion surfaces переносят `v12` first-poll contention attribution без guessed blocker claims (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v12` bounded `first_poll_contention_attribution` в человекочитаемом виде.

Human-readable projection MUST:
- показывать `first_poll_contention_attribution` рядом с existing `v11` first-poll / first-wake split;
- называть этот signal server-visible contender fact, а не "точным виновником";
- явно деградировать на `v11`, не выдумывая `first_poll_contention_attribution`;
- не подменять missing `v12` server attribution client-side probes, correlation heuristics или free-text summary.

#### Scenario: Panel и clipboard показывают видимый contender class рядом с existing ingress facts
- **GIVEN** extension получает completion timeline `v12` с bounded `first_poll_contention_attribution`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый contender fact рядом с existing dispatch, pre-service-scope, first-poll / first-wake и pre-method facts
- **AND** оператор может увидеть server-visible contender class без открытия raw JSON

#### Scenario: Incident bundle summary переносит `v12` contention facts без overclaim
- **GIVEN** incident bundle строится по `v12` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts из `first_poll_contention_attribution`
- **AND** derived handoff не переименовывает contender class в точный blocking request, request id или URI

#### Scenario: Extension явно деградирует на `v11`
- **GIVEN** connected server возвращает completion timeline `v11`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `first_poll_contention_attribution`
- **AND** человекочитаемый output явно отмечает, что bounded contender attribution unavailable by design for `contract=v11`

### Requirement: Existing completion surfaces различают ingress и query-body dominance без invented findings (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST использовать authoritative server stages для различения ingress bottleneck и query-body bottleneck.

Human-readable projection MUST:
- строиться только из bounded authoritative fields/stages и локальных bounded status markers;
- не публиковать verdict `adapter_before_dispatch_dominant`, если authoritative `dominant_stage` или visible `stages` показывают dominance внутри `query_bundle*`;
- использовать canonical bounded verdict vocabulary:
  - `query_bundle_dominant`
  - `query_bundle_pool_wait_dominant`
  - `query_bundle_deps_and_file_snapshot_dominant`
  - `query_bundle_owner_hint_dominant`
  - `query_bundle_ir_query_dominant`
  - `query_bundle_ir_retry_dominant`
  - `query_bundle_other_dominant`;
- переносить `query_bundle` dominance в человекочитаемом виде для panel, clipboard и incident summary, если connected server возвращает `v20` payload;
- явно деградировать на `v19`, не выдумывая detailed `query_bundle_pool_wait`, `query_bundle_ir_query` или equivalent split.

Если query-body leaf verdict публикуется, surfaces SHOULD также публиковать umbrella verdict `query_bundle_dominant`.
Query-body verdicts MUST иметь precedence над ingress-only verdicts для того же trace.

#### Scenario: Panel и clipboard не обвиняют adapter ingress при dominant query-body stage
- **GIVEN** extension получает completion timeline `v20`, где `adapter_to_dispatch_wait_ms` положителен, но authoritative `dominant_stage` находится в `query_bundle*`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** human-readable output не публикует `adapter_before_dispatch_dominant`
- **AND** output показывает truthful query-body dominance рядом с existing ingress facts

#### Scenario: Incident bundle summary переносит query-body root cause без guessed reconstruction
- **GIVEN** incident bundle строится по `v20` payload с detailed `query_bundle` stages
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded query-body stage facts и derived verdict
- **AND** summary не заменяет их guessed ingress bottleneck

#### Scenario: Extension явно деградирует на `v19`
- **GIVEN** connected server возвращает completion timeline `v19`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает detailed `query_bundle` split
- **AND** человекочитаемый output явно отмечает, что truthful query-body breakdown unavailable by design for `contract=v19`

### Requirement: Existing completion surfaces переносят `v21` post-response gap split без guessed root cause (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v21` flush-aware server egress split и новый client probe receive/resolve split в человекочитаемом виде.

Human-readable projection MUST:

- показывать `response_ready_to_flush_wait_ms`, если connected server возвращает `v21` payload с flush-aware boundary;
- при deterministic correlation и наличии нового probe split показывать отдельно `transport_to_client_receive_wait_ms`, `client_receive_to_resolve_wait_ms` и existing `client_post_response_ms`;
- сохранять existing `client_to_transport_wait_ms` как отдельный ingress bucket;
- MAY сохранять compatibility umbrella вроде `server_to_client_post_response_ms`, но MUST NOT использовать её как единственный evidence bucket, если новый split доступен;
- явно деградировать на `v20` и на legacy probe paths, не выдумывая flush или raw-receive boundaries.

#### Scenario: Panel и clipboard показывают split post-response tail

- **GIVEN** extension получает completion timeline `v21`
- **AND** correlated probe содержит raw receive и promise resolve milestones
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** output показывает server egress wait отдельно от transport-after-flush и client-after-receive waits
- **AND** оператору не нужно читать raw JSON, чтобы увидеть этот split

#### Scenario: Incident bundle summary не обвиняет одну сторону при incomplete split

- **GIVEN** connected server возвращает `v20` payload или correlated probe не имеет raw receive boundary
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary явно отмечает, что post-response gap split unavailable by design для этой evidence version
- **AND** derived handoff не переименовывает opaque tail в точный server-side или client-side виновник

