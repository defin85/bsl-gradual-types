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

