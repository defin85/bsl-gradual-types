# bsl-intellisense-v2 Specification

## Purpose
TBD - created by archiving change define-bsl-intellisense-v2. Update Purpose after archive.
## Requirements
### Requirement: IntelliSense v2 обеспечивает IDE‑grade completion по выражениям (MUST)
Система SHALL обеспечивать completion v2, который корректно работает для member access в выражениях и цепочках, включая неполный код:
- `Идентификатор.`
- `Вызов().`
- `Коллекция[...].`
- `(expr).`
- цепочки вида `a.b().c[d].e.`

#### Scenario: Completion работает на неполном коде
- **GIVEN** пользователь набирает `expr.` и код может быть синтаксически неполным
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** система извлекает receiver‑выражение и возвращает релевантные candidates без зависаний и с предсказуемым fallback

### Requirement: Инкрементальность и корректность позиций в v2 pipeline (MUST)
Система SHALL обеспечивать согласованность позиций между LSP (UTF‑16), внутренними byte offsets и tree‑sitter incremental parsing, чтобы completion не деградировал после `didChange`.

#### Scenario: Серия didChange не ломает completion
- **GIVEN** пользователь выполняет серию правок (включая Unicode) и IDE шлёт `didChange`
- **WHEN** IDE запрашивает completion после правок
- **THEN** система использует актуальный снапшот и выдаёт корректные результаты (без mixed state)

### Requirement: Интеграция stdlib + metadata как first‑class сценарий (MUST)
Система SHALL поддерживать completion для stdlib и метаданных 1С (минимум: `Документы.`/`Справочники.` и фасеты), как описано в roadmap IntelliSense v2.

#### Scenario: Completion по метаданным
- **GIVEN** загружены метаданные конфигурации
- **WHEN** IDE запрашивает completion для `Документы.`
- **THEN** система возвращает релевантные элементы метаданных (например, имена документов) с корректными деталями

### Requirement: Однозначный resolve completion candidates (MUST)
Система SHALL обеспечивать однозначный resolve выбранного completion item (без угадывания по label), используя стабильный идентификатор кандидата.

#### Scenario: Resolve не путает кандидатов
- **GIVEN** два completion item имеют похожие `label`
- **WHEN** IDE вызывает `completionItem/resolve` для одного из них
- **THEN** система разрешает именно выбранный item по стабильному идентификатору кандидата

### Requirement: Регрессионные тесты полноты и VS Code‑паттернов (MUST)
Система SHALL иметь тестовый набор, который фиксирует полноту completion (матрица выражений × источники) и воспроизводит VS Code‑паттерны `didChange` → completion.

#### Scenario: Регрессия полноты воспроизводима одной командой
- **GIVEN** в коде изменён completion pipeline
- **WHEN** запускаются тесты IntelliSense v2
- **THEN** регрессии полноты и инкрементальности воспроизводимы локально и дают детерминированный отчёт

### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать `bsl-analysis-v2` как единственный pipeline вывода типов для IDE‑функций (completion/hover/signatureHelp/definition/diagnostics).
Legacy‑пути вывода типов MUST быть удалены (не поддерживаются).

#### Scenario: Hover и completion используют один и тот же источник типовой информации
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает hover и completion в одной и той же позиции/контексте
- **THEN** ответы должны опираться на единый v2 snapshot и согласованный набор правил (без альтернативных inference путей)

### Requirement: Completion resolve не имеет legacy fallback (MUST)
Система MUST требовать `candidate_id` в `completionItem/resolve` и MUST NOT иметь fallback‑резолвинг по `kind/owner_type` или другим эвристикам.

#### Scenario: Completion resolve использует только candidate_id
- **GIVEN** IDE получает completion items от сервера
- **WHEN** IDE вызывает `completionItem/resolve` для любого item
- **THEN** сервер использует только `candidate_id` для резолва detail/documentation/snippet
- **AND** сервер не имеет legacy веток резолвинга без `candidate_id`

### Requirement: `bsl-semantic` удалён, AST→IR является частью v2 архитектуры (MUST)
Система MUST не содержать workspace crate `bsl-semantic`.
AST→IR конвертация (если требуется) MUST быть реализована как часть v2 архитектуры (внутри `bsl-analysis-v2` или в отдельном нейтральном crate без самостоятельного inference).

#### Scenario: Сборка не зависит от `bsl-semantic`
- **GIVEN** разработчик делает чистый checkout репозитория
- **WHEN** он собирает `bsl-analysis-v2` и `bsl-backend`
- **THEN** сборка не требует `bsl-semantic` как зависимости и не использует `bsl_semantic::*`

### Requirement: Минимальный IR является внутренним артефактом v2 и не выполняет inference (MUST)
Система MUST использовать минимальный IR как внутренний артефакт v2 pipeline (queries) для обеспечения IDE‑фич, но IR MUST NOT выполнять вывод типов или содержать альтернативные эвристики inference.

Минимальный IR MUST включать:
- модель scope/symbols (процедуры/функции/параметры/локальные переменные) с диапазонами объявлений,
- нормализованную модель выражений для completion v2 (`Identifier`, `MemberAccess`, `Call`, `IndexAccess`, `Grouping`),
- привязку узлов/выражений к тексту через byte offsets и единый слой позиционирования до LSP UTF‑16.

Идентификаторы узлов IR (например, `ExprId`) MUST быть валидны только в рамках одного v2 snapshot/revision.

#### Scenario: Единый inference поверх IR без обходов и фолбеков
- **GIVEN** IDE запрашивает completion/hover/definition для одного snapshot
- **WHEN** система вычисляет промежуточные данные
- **THEN** IR используется только для структурирования кода и извлечения связей/позиции
- **AND** вывод типов выполняется v2 pipeline поверх IR с использованием deps snapshot (без отдельного «эвристического» пути)

### Requirement: IR хранит позиции только в byte offsets (MUST)
Система MUST хранить ranges IR узлов как byte offsets и MUST NOT хранить line/column координаты внутри IR.

#### Scenario: LSP позиции получаются через конвертацию byte offsets
- **GIVEN** IR узлы имеют byte offsets
- **WHEN** сервер возвращает LSP ranges/positions в hover/diagnostics/definition
- **THEN** сервер конвертирует byte offsets в UTF‑16 позиции через v2 line-index слой

### Requirement: Non-LSP клиенты используют v2-only источники данных (MUST)
Система MUST обеспечивать, что Web API, CLI и `bsl-agent` используют v2-only источники данных
и не имеют альтернативных inference путей вне `bsl-analysis-v2`/deps snapshot.

#### Scenario: Web API не использует отдельный inference фасад
- **GIVEN** пользователь вызывает Web API эндпоинт (search/types/details/etc.)
- **WHEN** сервер формирует ответ
- **THEN** сервер использует только v2 deps snapshot (`SemanticDeps`) и/или v2 queries
- **AND** в коде Web API нет использования `TypeInferenceService`

#### Scenario: CLI не использует legacy AnalysisEngine
- **GIVEN** пользователь запускает CLI команду анализа
- **WHEN** CLI вычисляет diagnostics/семантику
- **THEN** используется `AnalysisHostV2`/`AnalysisV2` и v2 queries
- **AND** CLI не использует `bsl_shared::engine::AnalysisEngine`

#### Scenario: bsl-agent не использует отдельный inference фасад
- **GIVEN** клиент вызывает операции агента, требующие типовой информации
- **WHEN** агент вычисляет ответ
- **THEN** используется v2 deps snapshot (`SemanticDeps`) и/или v2 queries
- **AND** в коде агента нет использования `TypeInferenceService`

### Requirement: v2 pipeline предоставляет CFG для flow-sensitive анализа (MUST)
Система MUST строить и включать control-flow graph (CFG) в v2 pipeline таким образом, чтобы flow-sensitive анализ (type narrowing / null-safety) мог выполняться на основе v2 snapshot без альтернативных путей построения CFG.

#### Scenario: Flow-sensitive анализ доступен из v2 snapshot
- **GIVEN** пользователь работает с `.bsl` файлом в IDE
- **WHEN** система строит v2 snapshot для hover/completion/diagnostics
- **THEN** CFG присутствует в структуре программы (или доступен через стабильный API v2), и flow-sensitive логика использует этот CFG (без fallback на отдельные CFG/анализаторы вне v2)

### Requirement: В репозитории не существует двусмысленного `ControlFlowGraph` между слоями (MUST)
Система MUST избегать ситуации, когда в разных слоях (IR vs Domain) одновременно существуют разные типы с одинаковым именем `ControlFlowGraph`, если они оба используются/экспортируются как часть публичного API.

#### Scenario: Потребители однозначно импортируют CFG
- **GIVEN** разработчик пишет код, который использует CFG для анализа
- **WHEN** он импортирует `ControlFlowGraph`
- **THEN** импорт однозначен (нет двух разных `ControlFlowGraph` в публичной поверхности), либо используются явно разные имена (`Ir*`/`Domain*`) с понятной семантикой

### Requirement: Flow-sensitive режим в v2 является opt-in и согласован между интерфейсами (MUST)
Система MUST поддерживать flow-sensitive режим (type narrowing и null-safety) во всех v2 интерфейсах (IDE/LSP, Web API, MCP),
но MUST NOT включать его по умолчанию.

Flow-sensitive вычисления MUST выполняться только при явном включении effective флага/настройки, чтобы не ухудшать производительность по умолчанию.

#### Scenario: Flow-sensitive выключен по умолчанию
- **GIVEN** клиент использует IDE/Web API/MCP без явного включения flow-sensitive режима
- **WHEN** выполняются hover/completion/diagnostics/type-at-position запросы
- **THEN** система не выполняет flow-sensitive вычисления и возвращает результаты на основе базовых v2 queries

#### Scenario: Flow-sensitive включён и влияет на результаты
- **GIVEN** клиент явно включил flow-sensitive режим
- **WHEN** система отвечает на hover/completion/diagnostics/type-at-position запросы
- **THEN** ответы используют flow-sensitive результаты на основе CFG и v2 snapshot/queries

### Requirement: `SemanticProgram.cfg` всегда присутствует в v2 snapshot (MUST)
Система MUST всегда включать CFG в v2 snapshot как `SemanticProgram.cfg = Some(ControlFlowGraph)`, даже если в файле нет исполняемых конструкций.

Минимальный CFG в таком случае MUST содержать:
- узлы `Entry` и `Exit`,
- ребро `Entry -> Exit`.

#### Scenario: CFG присутствует для файлов без исполняемых конструкций
- **GIVEN** пользователь открывает `.bsl` файл, который содержит только объявления (или пустые тела процедур/функций)
- **WHEN** система строит v2 snapshot (IR) для hover/completion/diagnostics
- **THEN** `SemanticProgram.cfg` присутствует и содержит как минимум `Entry -> Exit`

### Requirement: Привязка “позиция → CFG узел” детерминирована и bias-aware (MUST)
Система MUST иметь единый детерминированный алгоритм выбора CFG узла по byte offset, используемый всеми flow-sensitive consumers.

Алгоритм MUST:
- выбирать “самый специфичный” (наиболее узкий) узел, чей span содержит позицию;
- поддерживать bias для случаев, когда позиция находится на границе токена (например, completion на `.` предпочитает выражение слева).

#### Scenario: Completion на `.` выбирает владельца слева
- **GIVEN** пользователь вводит `x.` внутри условной ветки
- **WHEN** IDE запрашивает completion в позиции сразу после `.`
- **THEN** система выбирает CFG узел/контекст, соответствующий выражению слева (`x`), а не произвольный соседний узел
- **AND** результат детерминирован при повторных запросах

### Requirement: v2 предоставляет корректный контракт “позиция → flow-sensitive тип” (MUST)
Система MUST иметь стабильный механизм получения flow-sensitive `TypeResolution` для byte offset позиции в документе,
чтобы hover/completion/signatureHelp/definition могли использовать уточнённый тип в текущем control-flow контексте.

Реализация SHOULD быть локальной по области анализа (например, CFG-per-body), чтобы минимизировать стоимость вычислений.

#### Scenario: Type-at-position учитывает narrowing в then-ветке
- **GIVEN** переменная имеет широкий/nullable тип до условия
- **WHEN** курсор находится внутри then-ветки после type guard (например, `x <> Неопределено` / `ТипЗнч(x)=...`)
- **THEN** flow-sensitive `type-at-position` возвращает уточнённый тип (narrowed) для `x`

### Requirement: Null-safety diagnostics добавляются только при включённом flow-sensitive режиме (MUST)
Система MUST добавлять null-safety diagnostics, вычисленные на основе CFG и flow-sensitive контекста, в v2 diagnostics pipeline
только при включённом flow-sensitive режиме.

#### Scenario: Null-safety предупреждение появляется только при включённом режиме
- **GIVEN** код содержит потенциальный null dereference по CFG (receiver может быть null/undefined)
- **WHEN** запрашиваются diagnostics при включённом flow-sensitive режиме
- **THEN** система возвращает diagnostics, включающие null-safety предупреждения
- **AND** при выключенном режиме эти предупреждения отсутствуют

### Requirement: Flow-sensitive null-safety учитывает null-check в заголовках циклов (MUST)
Система MUST учитывать null-check условия в заголовках циклов (например, `Пока x <> Null Цикл`) при выполнении flow-sensitive null-safety анализа.

#### Scenario: Null-check в `Пока` подавляет warning в теле
- **GIVEN** переменная `x` потенциально nullable
- **WHEN** код содержит цикл `Пока x <> Null Цикл ... x.Method() ... КонецЦикла`
- **THEN** flow-sensitive null-safety не выдаёт предупреждение о возможном Null‑dereference для `x.Method()` внутри тела цикла

### Requirement: v2 выводит return-тип локальных функций текущего модуля (MUST)
Система MUST выводить тип выражения вызова `F()` для функций, объявленных в текущем файле, включая:
- вызовы до объявления (forward reference);
- вызовы неэкспортных функций внутри модуля.

Выведенный тип MUST использоваться всеми v2 consumers (hover/completion/type-at-position/diagnostics) через единый v2 snapshot.

#### Scenario: Вызов локальной функции до объявления имеет корректный тип
- **GIVEN** в модуле есть `x = F();` до объявления `Функция F() ... Возврат "s"; КонецФункции`
- **WHEN** клиент запрашивает тип на позиции `F` в `x = F();`
- **THEN** система возвращает тип `Строка` для выражения вызова

### Requirement: v2 учитывает неявный `Возврат Неопределено` (soundness) (MUST)
Система MUST учитывать семантику implicit return: если возможен путь выполнения функции до `КонецФункции` без явного `Возврат`,
то итоговый return-тип MUST включать `Неопределено`.

#### Scenario: Ветвление без полного покрытия добавляет `Неопределено`
- **GIVEN** функция `F()` содержит `Если ... Тогда Возврат 1; КонецЕсли;` и может завершиться без `Возврат`
- **WHEN** клиент запрашивает тип выражения вызова `F()`
- **THEN** система возвращает `Число | Неопределено` (порядок не важен)

### Requirement: v2 объединяет типы из нескольких `Возврат` в union (MUST)
Система MUST объединять типы из нескольких операторов `Возврат` в `union` (вместо деградации в `Unknown`), с нормализацией и дедупликацией.

Это применяется, когда функция имеет несколько операторов `Возврат` с разными конкретными типами возвращаемых выражений.

#### Scenario: Два разных return дают union
- **GIVEN** функция `F()` возвращает либо `1`, либо `"x"` по разным веткам
- **WHEN** клиент запрашивает тип выражения вызова `F()`
- **THEN** система возвращает `Строка | Число`

### Requirement: Локальный return inference детерминирован и корректен при рекурсии (MUST)
Система MUST иметь детерминированный алгоритм вычисления локальных return-типов, корректно обрабатывающий взаимные вызовы и рекурсию,
не полагаясь только на “магические лимиты” итераций.

#### Scenario: Взаимная рекурсия не приводит к недетерминированности
- **GIVEN** две локальные функции `A()` и `B()`, которые вызывают друг друга
- **WHEN** клиент запрашивает тип выражения вызова `A()`
- **THEN** результат детерминирован между запусками и соответствует специфицированной политике решателя (например, `Unknown` при невозможности вывести более точный тип)

### Requirement: Транзитивный local return inference сохраняет структуру типов (MUST)
Система MUST обеспечивать транзитивный local return inference: если локальная функция `A()` возвращает результат вызова
другой локальной функции `B()` (например, `Возврат B();`), то return-тип `A()` MUST быть эквивалентен return-типу `B()`
**без потери структуры** (union остаётся union, а не “platform type со строковым именем”).

#### Scenario: Union return типа callee транзитивно сохраняется в caller
- **GIVEN** `B()` имеет несколько `Возврат` с разными конкретными типами (union)
- **AND** `A()` делает `Возврат B();`
- **WHEN** клиент запрашивает тип выражения вызова `A()` в позиции `A(`
- **THEN** система возвращает тип, эквивалентный return-типу `B()` (в том числе структурно `ResolutionResult::Union`)

### Requirement: Local return inference имеет специфицированную политику для взаимной рекурсии (MUST)
Система MUST иметь специфицированную политику для взаимной рекурсии (`A()` вызывает `B()`, `B()` вызывает `A()`).
Local return inference MUST:
- завершаться (не зависать) без опоры на “магические лимиты” итераций;
- возвращать детерминированный результат между запусками;
- следовать явно задокументированной policy (например: если точный тип не выводится, результат `Unknown`).

#### Scenario: Взаимная рекурсия детерминирована и завершает вычисление
- **GIVEN** две локальные функции `A()` и `B()`, которые вызывают друг друга
- **WHEN** клиент запрашивает тип выражения вызова `A()`
- **THEN** результат детерминирован между запусками
- **AND** вычисление local return inference завершается

### Requirement: Синтаксические diagnostics в IDE улучшаются rule-based post-processing (MUST)
Система MUST применять rule-based post-processing для синтаксических diagnostics, чтобы улучшать message и span
для распознаваемых паттернов ошибок.

Post-processing MUST:
- запускаться **только если** парсер уже вернул синтаксические ошибки (например, `ParseError`, `InvalidSyntax`, `MissingToken`);
- не менять грамматику языка и не “принимать” неверный синтаксис;
- быть детерминированным (одинаковый текст → одинаковый набор diagnostics).
- следовать политике rewrite-only: для строки с ошибкой возвращать не более 1 синтаксической диагностики (приоритет — улучшенная).

#### Scenario: Улучшение синтаксической ошибки не создаёт FP на валидном коде
- **GIVEN** валидный BSL код без синтаксических ошибок
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система не добавляет rule-based diagnostics

### Requirement: Диагностика для `Для ... По ... Шаг ... Цикл` указывает на первопричину (MUST)
Система MUST распознавать распространённый неверный паттерн `Для ... По ... Шаг ... Цикл` и выдавать диагностику,
которая:
- указывает span на ключевое слово `Шаг` (а не на произвольный “токен падения” парсера),
- сообщает, что в BSL нет синтаксиса `Шаг <expr>` в цикле `Для`,
- подсказывает корректный вариант (например, обратный обход `Для i = ... По 0 Цикл`).

#### Scenario: LLM-ошибка с `Шаг -1` диагностируется с понятным сообщением
- **GIVEN** код содержит строку `Для Индекс = ТЗ.Количество() - 1 По 0 Шаг -1 Цикл`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система возвращает `InvalidSyntax` (или эквивалентный error code) со span на `Шаг`
- **AND** сообщение объясняет, что `Шаг` не является частью синтаксиса `Для` в BSL

### Requirement: Ошибки заголовка `Если` указывают на отсутствие `Тогда` (MUST)
Система MUST улучшать сообщение/span для частых ошибок заголовка `Если`, в частности когда условие указано,
но отсутствует ключевое слово `Тогда`.

#### Scenario: `Если` без `Тогда` диагностируется корректно
- **GIVEN** код содержит строку `Если x = 1` и далее идёт тело без `Тогда`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система возвращает одну диагностику на строку заголовка `Если`
- **AND** сообщение указывает на отсутствие `Тогда`

### Requirement: Ошибки структуры `Попытка` указывают на незакрытый блок или отсутствующий `Исключение` (MUST)
Система MUST улучшать сообщение/span для частых ошибок структуры `Попытка`, включая:
- отсутствующий `КонецПопытки`;
- отсутствующий `Исключение` там, где он требуется структурой блока.

#### Scenario: Незакрытая `Попытка` диагностируется как незакрытый блок
- **GIVEN** код содержит `Попытка ...` без `КонецПопытки`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система возвращает одну диагностику на строку (в приоритете — улучшенная)
- **AND** сообщение говорит, какой ключевой элемент блока отсутствует

### Requirement: Итоговые синтаксические diagnostics нормализуются единым pipeline (MUST)
Система MUST нормализовывать **все** синтаксические diagnostics, возвращаемые в IDE/LSP, единым детерминированным pipeline:
- объединить diagnostics из parser (tree-sitter `ERROR`/missing) и IDE‑эвристик (semicolon, неполный `Новый`, и т.п.);
- применить rewrite/enrich правила (без изменения грамматики);
- применить строгий line-cap: **не более 1 синтаксической диагностики на строку** в итоговом списке;
- обеспечить детерминированный порядок diagnostics.

#### Scenario: Эвристики не нарушают line-cap при наличии parser errors
- **GIVEN** код содержит parser syntax error на строке `X`
- **AND** на той же строке `X` срабатывает IDE‑эвристика (например, semicolon)
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система возвращает ровно 1 синтаксическую диагностику на строку `X`
- **AND** в приоритете остаётся более конкретная/структурная диагностика

### Requirement: Обобщённая диагностика заголовка `Для` строго сообщает “ожидается `Цикл`” (MUST)
Система MUST улучшать диагностику заголовка `Для` в случае лишнего токена между `По <expr>` и `Цикл`:
- сообщение MUST содержать “После `По <выражение>` ожидается `Цикл`”;
- span MUST указывать на **первый** неожиданный токен между `По` и `Цикл`, даже если это мусор (идентификатор/число/символ).

#### Scenario: Несколько мусорных токенов — подсвечивается первый
- **GIVEN** код содержит строку `Для i = 10 По 0 abc def Цикл`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система возвращает `InvalidSyntax` со span на `abc`
- **AND** сообщение содержит “ожидается `Цикл`”

### Requirement: Related‑информация для незакрытой `Попытка` доступна в IDE (MUST)
Система MUST обеспечивать related‑информацию для диагностики незакрытого блока `Попытка`,
чтобы IDE могла показать “Начало блока: Попытка” даже если исходная ошибка пришла как общий `ParseError`.

#### Scenario: Незакрытая `Попытка` содержит related на начало блока
- **GIVEN** код содержит `Попытка ... Исключение ...` без `КонецПопытки`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** диагностика содержит related‑точку “Начало блока: Попытка” со span на `Попытка`

### Requirement: Правила не срабатывают на ключевые слова внутри строк/комментариев (MUST)
Система MUST предотвращать ложные срабатывания rewrite‑правил на ключевые слова,
которые встречаются внутри строковых литералов или комментариев.

#### Scenario: `Шаг` в строке не триггерит rewrite правила `Для`
- **GIVEN** строка содержит `Для i = 10 По 0 "Шаг" Цикл`
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система не выдаёт диагностику, которая утверждает, что `Шаг <expr>` является неверной клаузой `Для`

### Requirement: Masking `//` применяется до конца строки (MUST)
Система MUST применять masking для `//` комментариев до конца строки, даже если анализируемый фрагмент текста
является многострочным срезом (например, содержимое `ParseError` span, включающее newline).

#### Scenario: `// Шаг` в комментарии не триггерит правило `Шаг <expr>`
- **GIVEN** код содержит заголовок `Для` и комментарий `// Шаг` в той же области
- **WHEN** IDE запрашивает синтаксические diagnostics
- **THEN** система не выдаёт диагностику, утверждающую, что `Шаг <expr>` является неверной клаузой `Для` из комментария

### Requirement: Все runtime tunables `BSL_*` управляемы без рестарта LSP
Система SHALL позволять управлять runtime `BSL_*` параметрами через VS Code settings, и применять их без рестарта LSP процесса.

#### Scenario: Изменение debounce влияет без рестарта
- **GIVEN** LSP сервер запущен
- **WHEN** пользователь меняет настройку, соответствующую `BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS`
- **THEN** последующие diagnostics используют новый debounce без рестарта

### Requirement: Non-member completion учитывает лексическую видимость локальных символов (MUST)
Система MUST формировать локальные candidates для non-member completion на основе лексической области видимости в позиции курсора.

Для локальных symbols MUST применяться правила:
- в выдачу попадают только symbols из лексической области (scope chain), содержащей позицию курсора;
- symbols, объявленные после позиции курсора, MUST NOT предлагаться;
- при затенении одинаковых имён MUST использоваться ближайшее объявление (nearest scope wins);
- локальные symbols, появляющиеся через присваивание идентификатору (`x = ...`), становятся видимыми начиная с позиции этого присваивания.

#### Scenario: Локальная переменная из блока не видна вне блока
- **GIVEN** в процедуре переменная объявлена/инициализирована внутри ветки `Если`
- **WHEN** IDE запрашивает non-member completion после `КонецЕсли` вне этой ветки
- **THEN** переменная из внутреннего блока не возвращается в completion candidates

#### Scenario: Символ после курсора не попадает в completion
- **GIVEN** курсор стоит в строке до объявления локальной переменной
- **WHEN** IDE запрашивает non-member completion
- **THEN** переменная, объявленная ниже по коду, отсутствует в выдаче

#### Scenario: Затенение имён выбирает ближайший symbol
- **GIVEN** в внешнем и вложенном блоке есть локальные symbols с одинаковым именем
- **WHEN** IDE запрашивает non-member completion внутри вложенного блока
- **THEN** completion возвращает symbol из ближайшего (вложенного) scope

#### Scenario: Неявный локал из присваивания виден после присваивания
- **GIVEN** в процедуре есть `Локал = Новый Массив;`
- **WHEN** IDE запрашивает non-member completion на строке ниже этого присваивания
- **THEN** symbol `Локал` присутствует в candidates
- **AND** при запросе completion до строки присваивания symbol `Локал` отсутствует

### Requirement: Источник локальных symbols для completion синхронизирован с v2 snapshot (MUST)
Система MUST формировать локальные completion candidates из того же v2 snapshot (IR + scopes), который используется для остальных v2 IDE-операций в текущей ревизии документа.

Система MUST NOT использовать файловый symbol index как первичный источник локальных symbols для non-member completion.

#### Scenario: Локалы другой процедуры не попадают в completion текущей позиции
- **GIVEN** в файле есть две процедуры с разными локальными переменными
- **WHEN** IDE запрашивает non-member completion внутри первой процедуры
- **THEN** локальные symbols второй процедуры не возвращаются в выдаче

### Requirement: v2 context-aware implicit symbols для модулей определяются по ModuleType/фасету (MUST)
Система MUST определять и подмешивать platform implicit symbols в v2 pipeline на основе модульного контекста (`ModuleType`) и фасета владельца метаданных через descriptor-based semantic model.

Система MUST представлять implicit типы структурно (descriptor-based), а не только строковым именем типа.
Descriptor MUST сохранять контекст, достаточный для детерминированного преобразования в `TypeResolution` (минимум: owner metadata, module context, required facet, form context при наличии).

Система MUST использовать единый descriptor-based контракт для:
- AST→IR symbol registration;
- type inference seeding;
- последующей семантической диагностики undeclared variable и member-resolution.

#### Scenario: Единые правила не расходятся между AST→IR и type inference
- **GIVEN** файл анализируется через v2 pipeline
- **WHEN** система строит symbols и type hints для одинакового snapshot
- **THEN** один и тот же implicit symbol считается объявленным и имеет согласованный тип во всех этапах pipeline
- **AND** при преобразовании descriptor -> `TypeResolution` сохраняется ожидаемый facet context

### Requirement: FormModule предоставляет фиксированный набор implicit symbols (MUST)
Для `FormModule` система MUST предоставлять следующие implicit symbols:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

Типы MUST вычисляться контекстно через descriptor-based модель:
- `ЭтотОбъект`, `ЭтаФорма`, `Форма` -> дескриптор контекста формы с user-facing представлением `Формы.<Коллекция>.<Объект>.<Форма>`;
- `Объект` -> form-data descriptor, связанный с applied object фасетом владельца для guaranteed members;
- `Элементы` -> дескриптор контейнера элементов формы с user-facing представлением `ЭлементыФормы.<Коллекция>.<Объект>.<Форма>`;
- `Параметры` -> `Структура`.

Система MUST NOT использовать `ДанныеФормыОбъект.*` как внутренний semantic source of truth для `FormModule.Объект`.
Canonical semantic interpretation для `FormModule.Объект` MUST соответствовать form-data semantics (`ДанныеФормыСтруктура`), даже если user-facing label использует owner object facet.

#### Scenario: `ПриСозданииНаСервере` использует `ЭтотОбъект` и `Параметры` без undeclared diagnostic
- **GIVEN** модуль формы документа содержит вызов `...ПриСозданииНаСервереДокумент(ЭтотОбъект, Параметры)`
- **WHEN** клиент запрашивает semantic diagnostics
- **THEN** система не возвращает diagnostics `Необъявленная переменная` для `ЭтотОбъект` и `Параметры`

### Requirement: ManagerModule содержит implicit `ЭтотОбъект` и `Объект` менеджер-фасета (MUST)
Для `ManagerModule` система MUST предоставлять implicit symbols `ЭтотОбъект` и `Объект` с типом manager-фасета владельца метаданных.

#### Scenario: В ManagerModule `Объект` не считается необъявленным
- **GIVEN** код в `ManagerModule` использует `Объект` или `ЭтотОбъект`
- **WHEN** выполняется v2 семантическая диагностика
- **THEN** система не возвращает `Необъявленная переменная` для этих идентификаторов

### Requirement: Object/RecordSet modules содержат implicit `ЭтотОбъект` и `Объект` object-фасета (MUST)
Для `ObjectModule` и `RecordSetModule` система MUST предоставлять implicit symbols `ЭтотОбъект` и `Объект` с типом object-фасета владельца.

#### Scenario: В ObjectModule `ЭтотОбъект` не считается необъявленным
- **GIVEN** код в `ObjectModule` обращается к `ЭтотОбъект`
- **WHEN** выполняется v2 семантическая диагностика
- **THEN** система не возвращает `Необъявленная переменная 'ЭтотОбъект'`

### Requirement: Директивы `*БезКонтекста` отключают context-bound symbols формы (MUST)
В процедурах/функциях с директивами `&НаСервереБезКонтекста` и `&НаКлиентеНаСервереБезКонтекста` система MUST считать context-bound symbols формы недоступными:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

#### Scenario: `&НаСервереБезКонтекста` не видит context-bound symbol
- **GIVEN** в модуле формы процедура помечена `&НаСервереБезКонтекста`
- **WHEN** внутри процедуры используется `ЭтотОбъект`
- **THEN** система возвращает диагностику о необъявленной переменной для `ЭтотОбъект`

### Requirement: Implicit-symbols в v2 MUST резолвиться контекстно по ModuleType
Система MUST определять типы implicit-symbols через единый контекстный резолвер, учитывающий `ModuleType`, владельца метаданных и директивы компиляции.

Для одинакового имени symbol (например, `Объект`) тип MUST зависеть от модульного контекста:
- `FormModule` -> form-data модель;
- `ManagerModule` -> manager facet владельца;
- `ObjectModule`/`RecordSetModule` -> object/recordset facet владельца.

#### Scenario: Один и тот же symbol `Объект` получает разные типы в разных модулях
- **GIVEN** есть `FormModule` и `ManagerModule` одного владельца метаданных
- **WHEN** v2 pipeline строит type hints для идентификатора `Объект`
- **THEN** тип `Объект` в `FormModule` соответствует form-data модели
- **AND** тип `Объект` в `ManagerModule` соответствует manager facet

### Requirement: Для `FormModule.Объект` v2 MUST использовать платформенную form-data модель
Система MUST представлять `Объект` в модуле формы через платформенную семантику form data (`ДанныеФормыСтруктура` и связанные form-data типы), а не через внутренний synthetic alias.

Система MUST поддерживать доступ к гарантированным членам applied object, релевантным для form-data контекста (включая `Ссылка` для документных форм).

#### Scenario: `Объект.Ссылка` в форме документа не даёт ложный `NonExistentProperty`
- **GIVEN** код формы документа обращается к `Объект.Ссылка`
- **WHEN** выполняется v2 semantic diagnostics
- **THEN** система не возвращает диагностику `Свойство 'Ссылка' не существует`

### Requirement: Legacy `ДанныеФормыОбъект.*` MUST быть удалён из user-facing v2 outputs
Система MUST NOT использовать или показывать `ДанныеФормыОбъект.*` в user-facing результатах v2 (`diagnostics`, `hover`, `completion`, `type-at-position`).

#### Scenario: Пользовательская выдача не содержит legacy alias
- **GIVEN** пользователь запрашивает hover и diagnostics для form-object выражений
- **WHEN** v2 pipeline возвращает результаты
- **THEN** в сообщениях и type labels отсутствует `ДанныеФормыОбъект.*`

### Requirement: Descriptor-aware member resolution для FormModule.Объект является детерминированным (MUST)
Для `FormModule.Объект` система MUST выполнять member-resolution через отдельный descriptor-aware provider chain в фиксированном порядке:
1. form shape (реквизиты формы и привязанные элементы/ТЧ по контексту формы),
2. guaranteed members applied object (например, `Ссылка` для документа),
3. applied facet fallback.

Система MUST деградировать в `InferredWeak` при отсутствии достаточных метаданных вместо ложных `NonExistentProperty`.

#### Scenario: `Объект.Ссылка` в форме документа не даёт ложный `NonExistentProperty`
- **GIVEN** код формы документа обращается к `Объект.Ссылка`
- **WHEN** выполняется v2 semantic diagnostics
- **THEN** система не возвращает диагностику `Свойство 'Ссылка' не существует`
- **AND** type-at-position для `Объект` резолвится через descriptor-based контекст формы

### Requirement: Legacy form-object alias не участвует в descriptor-based semantic contract (MUST)
Система MUST трактовать `ДанныеФормыОбъект.*` только как migration compatibility alias на входе/нормализации.

Система MUST NOT использовать `ДанныеФормыОбъект.*` как canonical semantic type в seed/inference/lookup и MUST NOT показывать его в user-facing результатах (`diagnostics`, `hover`, `completion`, `type-at-position`).

#### Scenario: Пользовательская выдача и внутренний контракт не используют legacy alias как canonical type
- **GIVEN** пользователь запрашивает hover/diagnostics/completion для form-object выражений
- **WHEN** v2 pipeline возвращает результаты
- **THEN** в сообщениях и type labels отсутствует `ДанныеФормыОбъект.*`
- **AND** internal implicit binding/member-resolution используют descriptor-based model, а не legacy alias

### Requirement: User-facing label policy для FormModule.Объект отделён от canonical semantics (MUST)
Система MUST применять dual-layer policy для `FormModule.Объект`:
- internal canonical semantics: form-data descriptor (`ДанныеФормыСтруктура` semantics),
- compact/standard user-facing label: owner object facet (`ДокументОбъект.X`, `СправочникОбъект.X`, и т.д.),
- detailed user-facing label: owner object facet + явная form-data пометка (например, `ДокументОбъект.X (данные формы: ДанныеФормыСтруктура)` или эквивалент).

Система MUST обеспечивать согласованность этой политики между `hover`, `diagnostics`, `completion` и `type-at-position`.

#### Scenario: Compact и detailed режимы отображают согласованные слои семантики
- **GIVEN** выражение `Объект` в `FormModule` документной формы
- **WHEN** пользователь запрашивает тип в compact/standard режиме
- **THEN** label показывает owner object facet (`ДокументОбъект.<ИмяДокумента>`)
- **AND WHEN** пользователь запрашивает detailed представление
- **THEN** вывод содержит owner object facet и явную form-data пометку

