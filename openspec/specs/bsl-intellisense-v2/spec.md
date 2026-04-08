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

Syntax extraction для неполного кода MAY использовать parse/syntax helpers, но semantic candidates для completion MUST происходить только из canonical IR snapshot текущей revision и его canonical derived completion artifacts.

Для completion допускается bounded set canonical current-revision artifacts:
- `CompletionHeadArtifact` — fast artifact для initial completion response;
- `ExactSemanticArtifact` (`derived semantic index`) — full exact semantic artifact для enriched completion и других interactive semantic операций.

Оба артефакта MUST:
- строиться только из canonical IR snapshot той же revision;
- invalidated по `(file_version, deps_id, settings_id)`;
- не использовать stale payload другой revision как substitute.

`CompletionHeadArtifact` для текущей revision MUST быть publishable и queryable независимо от ready-state `ExactSemanticArtifact` той же revision. Completion MUST NOT оставаться effectively `exact-only` только потому, что exact artifact ещё не достроен после нового `didChange`.

Если current-revision `CompletionHeadArtifact` и `ExactSemanticArtifact` недоступны, completion MUST работать fail-closed и MUST NOT синтезировать semantic candidates из stale cache, keyword fallback или альтернативного inference path.
Система MUST NOT возвращать semantic candidates другой revision под видом current-revision completion ответа.

#### Scenario: Completion после новой revision может вернуться из current-revision completion head artifact
- **GIVEN** пользователь только что создал новую requested revision через `didChange`
- **AND** exact semantic artifact текущей revision ещё не ready
- **AND** current-revision `CompletionHeadArtifact` уже ready
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает semantic completion response из `CompletionHeadArtifact` той же revision
- **AND** не использует stale semantic payload другой revision

#### Scenario: Недоступность current-revision completion artifacts не превращается в semantic fallback
- **GIVEN** для текущей revision недоступны и `CompletionHeadArtifact`, и `ExactSemanticArtifact`
- **WHEN** IDE запрашивает completion на позиции после `.`
- **THEN** сервер возвращает explicit empty/unavailable fail-closed response для этой revision
- **AND** сервер не возвращает stale, degraded или keyword-only semantic substitute

### Requirement: Инкрементальность и корректность позиций в v2 pipeline (MUST)
Система SHALL обеспечивать согласованность позиций между LSP (UTF-16), внутренними byte offsets и tree-sitter incremental parsing, чтобы completion не использовал semantic truth от другой revision после `didChange`.

Система SHALL гарантировать, что interactive semantic ответы после `didChange` опираются только на canonical artifacts текущей revision или fail-closed для этой revision.

#### Scenario: Первый completion после `didChange` не использует semantic truth предыдущей revision
- **GIVEN** пользователь вводит `expr.` и IDE отправляет `didChange` для новой версии документа
- **WHEN** IDE немедленно отправляет `textDocument/completion` в позиции после `.`
- **THEN** сервер отвечает exact semantic результатом для новой revision или fail-closed response для новой revision
- **AND** не использует stale semantic candidates от предыдущей revision как substitute

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
Система MUST использовать canonical IR как единственный semantic source of truth для IDE-функций (`completion`, `hover`, `signatureHelp`, `definition`, `diagnostics`, `type-at-position`).

Bounded set canonical derived semantic artifacts MUST строиться только из canonical IR snapshot:
- `CompletionHeadArtifact` — fast query artifact только для initial completion response;
- `ExactSemanticArtifact` (`derived semantic index`) — full semantic artifact для exact completion и остальных interactive semantic запросов.

Legacy-пути вывода типов MUST быть удалены (не поддерживаются), включая parse-result-based semantic inference paths, которые существуют параллельно canonical IR.

#### Scenario: Completion head и exact artifact используют один canonical snapshot
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает completion, а затем hover в том же current-revision контексте
- **THEN** completion head и exact semantic artifact опираются на один canonical IR snapshot той же revision
- **AND** не используют альтернативные semantic inference пути вне canonical IR contract

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
Для `FormModule.Объект` система MUST выполнять member-resolution через form-data-oriented provider chain без applied object facet fallback.

Детерминированная цепочка MUST включать только:
1. members данных формы (shape главного реквизита и связанных данных формы),
2. платформенные members form-data типа.

Система MUST NOT использовать provider-шаг applied object facet fallback для `FormModule.Объект`.

#### Scenario: Детерминированный form-data chain без applied facet fallback
- **GIVEN** `FormModule.Объект` в документной форме
- **WHEN** v2 pipeline строит members для hover/completion
- **THEN** используется только form-data-oriented chain
- **AND** applied object facet fallback не участвует в выдаче

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

### Requirement: Единый orchestration facade для всех v2 интерфейсов (MUST)
Система MUST выполнять v2 semantic orchestration через единый shared facade в `bsl-runtime` для всех интерфейсов (`LSP`, `web`, `MCP`).

Adapter-layer код MUST оставаться transport-oriented (LSP/HTTP/MCP mapping) и MUST NOT содержать production orchestration цепочки напрямую через ad-hoc `AnalysisHostV2` setup, ручное sequencing `wait/snapshot/query` и adapter-local ветки semantic pipeline.

#### Scenario: LSP, web и MCP используют один orchestration контракт
- **GIVEN** одинаковые входные данные (текст документа, deps snapshot, настройки, позиция)
- **WHEN** клиент запрашивает semantic операцию через LSP, web и MCP
- **THEN** операция выполняется через общий facade path с согласованной стадийной последовательностью
- **AND** различия между интерфейсами ограничены транспортным форматом ответа

### Requirement: Производительные политики v2 централизованы и наследуются всеми адаптерами (MUST)
Система MUST централизовать performance-sensitive политику в shared facade/runtime:
- lazy `parse_result`,
- cancellation policy для IR/syntax/semantic queries,
- bounded blocking/concurrency control,
- queue-wait и stage-latency observability.

Adapter-local reimplementation этих политик MUST NOT использоваться в production semantic path.

#### Scenario: Исправление lazy `parse_result` применяется сразу во всех интерфейсах
- **GIVEN** policy требует не выполнять `parse_result`, если IR недоступен
- **WHEN** первый semantic запрос выполняется через LSP, web и MCP
- **THEN** ни один интерфейс не запускает `parse_result` при отсутствующем IR
- **AND** outcome/latency метрики отражают согласованное поведение во всех интерфейсах

### Requirement: Drift-prevention через cross-interface parity и perf regression (MUST)
Система MUST иметь автоматические проверки, которые предотвращают расхождение поведения между `LSP`, `web` и `MCP`:
- semantic parity tests на общих fixture/snapshot,
- cold/warm perf regression tests на крупных модулях,
- observability parity checks для стадий v2 pipeline.

#### Scenario: Drift в одном интерфейсе блокируется тестами
- **GIVEN** изменение в semantic orchestration влияет только на один адаптер
- **WHEN** запускаются parity/perf regression проверки
- **THEN** проверка завершается ошибкой
- **AND** изменение не считается принятым до восстановления parity

### Requirement: LSP interactive операции v2 используют bounded wait + fail-closed freshness policy (MUST)
Для `completion`, `hover`, `signatureHelp` система MUST применять freshness policy:
- сначала пытаться обслужить `requested file version` по фактически `applied_version`;
- ждать не дольше `intellisense_v2_interactive_wait_budget_ms` (дефолт `120ms`, если ключ не задан);
- после исчерпания wait budget завершать запрос fail-closed для текущей revision без stale semantic substitute.

Runtime knob MUST валидироваться и приводиться к диапазону:
- `intellisense_v2_interactive_wait_budget_ms` в диапазон `[10, 2000]`.

Snapshot с несовпадающими `deps_id` или `settings_id`, а также snapshot предыдущей revision, MUST NOT использоваться как semantic substitute для interactive ответа.

Дополнительно для completion:
- completion MUST иметь head-first current-revision prepare path для first response;
- member-access completion MUST NOT требовать generic full `snapshot_with_deps` как обязательный prereq для `head_hit`, если current-revision head truth уже доступен;
- `prepare_stateful_operation` MAY использоваться для completion exact route и exact upgrade, но MUST NOT оставаться обязательной первой ступенью каждого member-access completion после нового `didChange`;
- если `CompletionHeadArtifact` ready внутри wait budget, completion MAY вернуть current-revision semantic response из lightweight head path;
- если `ExactSemanticArtifact` ready внутри wait budget, completion MAY использовать exact semantic response напрямую;
- если внутри wait budget не ready ни один current-revision completion artifact, completion MUST завершиться fail-closed.

#### Scenario: Head-first completion не застревает за heavy generic prepare
- **GIVEN** пользователь только что создал новую requested revision
- **AND** current-revision `CompletionHeadArtifact` уже ready
- **AND** exact semantic path еще не готов
- **WHEN** IDE запрашивает member-access completion
- **THEN** сервер возвращает current-revision first response из head-first path
- **AND** не делает heavy exact prepare обязательной ступенью перед этим ответом

#### Scenario: Exact-only операции остаются на heavy prepare
- **GIVEN** exact semantic artifact текущей revision нужен для `hover`, `definition` или `signatureHelp`
- **WHEN** IDE выполняет такую операцию
- **THEN** сервер использует exact stateful prepare
- **AND** не заменяет его lightweight completion path

### Requirement: Diagnostics publish остаётся strict latest-version и monotonic по ревизии (MUST)
Система MUST публиковать `diagnostics` только для актуальной requested version документа.

Результаты, вычисленные для stale версии или stale ревизии зависимостей, MUST NOT быть опубликованы и MUST NOT перезаписывать diagnostics более новой ревизии.

Ревизия для publish MUST валидироваться как минимум по:
- `file_version`;
- `deps_id`;
- `settings_id`.

#### Scenario: Вычисленный stale diagnostics не публикуется
- **GIVEN** diagnostics для версии `V` уже запущен
- **AND** до публикации приходит новая requested version `V+1`
- **WHEN** вычисление для `V` завершается позже
- **THEN** результат для `V` не публикуется в IDE
- **AND** публикуется только результат, соответствующий актуальной requested version

### Requirement: Revision-bound expensive queries дедуплицируются singleflight с корректным lifecycle (MUST)
Для одинакового ключа ревизии `(file_id, file_version, deps_id, settings_id, query_kind)` система MUST выполнять не более одного дорогого query одновременно и делиться результатом между конкурентными запросами.

`query_kind` MUST включать минимум:
- `parse_result`
- `syntax_diagnostics`
- `ir`

Followers MUST получать тот же терминальный outcome, что и leader (`success`, `empty`, `error`, `cancelled`) для данного flight.

Система MUST NOT выполнять автоматический повтор внутри того же flight при `error/cancelled`; новый flight может быть создан только новым входящим запросом на тот же ключ после завершения предыдущего.

#### Scenario: Параллельные completion и diagnostics делят один parse_result
- **GIVEN** два конкурентных запроса требуют `parse_result` для одного и того же ключа ревизии
- **WHEN** оба запроса обрабатываются одновременно
- **THEN** `parse_result` вычисляется один раз
- **AND** оба запроса получают согласованный результат этой единственной вычислительной операции

#### Scenario: Отмена follower не ломает shared вычисление
- **GIVEN** один запрос является `leader`, второй подключён как `shared follower` к тому же singleflight-ключу
- **AND** follower отменяется клиентом
- **WHEN** лидерное вычисление уже запущено
- **THEN** лидерное вычисление не прерывается из-за отмены follower
- **AND** запись in-flight singleflight очищается после завершения leader (success/error/cancel)

#### Scenario: Ошибка leader распространяется на followers без auto-retry в том же flight
- **GIVEN** для singleflight-ключа запущен leader
- **AND** к flight подключены followers
- **WHEN** leader завершается с ошибкой
- **THEN** followers получают тот же ошибочный outcome этого flight
- **AND** внутри текущего flight не запускается повторное вычисление
- **AND** новый leader может появиться только на следующий входящий запрос после очистки in-flight записи

### Requirement: CPU планирование отделяет interactive и background бюджеты с fairness-гарантией (MUST)
Система MUST планировать CPU-bound semantic работу так, чтобы background diagnostics не могли полностью занять вычислительную ёмкость, необходимую для интерактивных операций.

При общем числе permits `>= 2` система MUST резервировать как минимум:
- `1` permit для interactive-класса;
- `1` permit для background-класса.

Система MUST приоритизировать control-path orchestration операции (apply changes, wait-for-version coordination) относительно тяжёлых query-path задач.

Background-класс MUST NOT заимствовать interactive reserve при наличии interactive waiters.
Interactive-класс MAY заимствовать background reserve только когда background queue пуста и это не нарушает гарантированный минимум background-прогресса.

#### Scenario: Background load не вытесняет interactive waiters
- **GIVEN** в системе идёт интенсивный поток background diagnostics/query задач
- **AND** есть ожидающий интерактивный completion/hover запрос
- **WHEN** планировщик выбирает следующую задачу
- **THEN** интерактивный запрос получает слот без ожидания завершения всего background хвоста
- **AND** background не забирает interactive reserve при наличии interactive waiters

#### Scenario: Background сохраняет прогресс под interactive нагрузкой
- **GIVEN** идёт непрерывный поток interactive запросов
- **WHEN** запланирован diagnostics task
- **THEN** diagnostics получает минимум background-прогресс
- **AND** система не уходит в starvation diagnostics

### Requirement: Observability контракт отражает fail-closed/singleflight/priority поведение фиксированными ключами (MUST)
Система MUST предоставлять в observability snapshot следующие ключи метрик.

Counter keys:
- `intellisense_v2_interactive_wait_budget_exhausted_total`
- `intellisense_v2_interactive_stale_served_total`
- `intellisense_v2_interactive_knob_clamped_total`
- `intellisense_v2_singleflight_leader_total`
- `intellisense_v2_singleflight_shared_total`
- `intellisense_v2_runtime_queue_wait_interactive_total`
- `intellisense_v2_runtime_queue_wait_background_total`
- `intellisense_v2_runtime_exec_interactive_total`
- `intellisense_v2_runtime_exec_background_total`
- `intellisense_v2_completion_stale_fallback_total`
- `intellisense_v2_completion_fallback_unavailable_total`
- `intellisense_v2_revision_lag_sample_total`

Histogram keys:
- `intellisense_v2_singleflight_wait_ms`
- `intellisense_v2_runtime_queue_wait_interactive_ms`
- `intellisense_v2_runtime_queue_wait_background_ms`
- `intellisense_v2_runtime_exec_interactive_ms`
- `intellisense_v2_runtime_exec_background_ms`
- `intellisense_v2_revision_lag_versions`

#### Scenario: Метрики показывают lag и fail-closed причину
- **GIVEN** completion завершается fail-closed из-за отставания applied revision
- **WHEN** запрашивается snapshot observability
- **THEN** snapshot содержит обязательные wait-budget/fallback-unavailable/lag ключи
- **AND** `revision_lag_versions` отражает факт lag-driven miss, а stale guardrail counters остаются нулевыми

### Requirement: Interactive latency quality gate фиксирует warm-path SLO (MUST)
Система MUST выполнять интерактивный latency gate для completion v2 в двух профилях одного тестового цикла:
- `large` профиль (реальный тяжёлый модуль);
- `small` профиль (контрольный лёгкий модуль).

Gate MUST использовать versioned baseline artifact и рассчитывать ratio к baseline для каждого профиля.

Для `large` warm-path MUST выполняться оба условия:
- `p95(intellisense_v2_wait_for_file_version_completion_ms) <= 0.60 * baseline_large_wait_for_file_version_p95_ms`;
- `p95(completion_duration_ms) <= 0.75 * baseline_large_completion_duration_p95_ms`.

Для `small` warm-path MUST выполняться non-regression условие:
- `p95(completion_duration_ms) <= 1.25 * baseline_small_completion_duration_p95_ms`.

Дополнительно quality gate MUST проверять устойчивость completion outcomes для каждого профиля:
- `completion_cancelled_rate <= 0.10`, где `completion_cancelled_rate = intellisense_v2_completion_result_total_cancelled / completion_total`;
- прогон каждого профиля MUST включать не менее `50` последовательных completion-запросов в рамках одной сессии.

#### Scenario: Large profile показывает objective ускорение относительно baseline
- **GIVEN** выполнен warm-path прогон `large` профиля и доступен baseline artifact
- **WHEN** рассчитываются ratio для `wait_for_file_version_completion_ms` и `completion_duration_ms`
- **THEN** оба ratio укладываются в целевые границы (`<=0.60` и `<=0.75`)
- **AND** `completion_cancelled_rate` не превышает 10%

#### Scenario: Small profile не деградирует при оптимизации large profile
- **GIVEN** выполнен warm-path прогон `small` профиля и доступен baseline artifact
- **WHEN** рассчитывается `completion_duration_ms` ratio
- **THEN** ratio не превышает `1.25`
- **AND** `completion_cancelled_rate` не превышает 10%

### Requirement: LSP runtime отслеживает received и applied ревизии файла раздельно (MUST)
Система MUST вести для каждого открытого `file_id` две независимые ревизии:
- `received_version`: последняя версия, полученная transport-слоем из `didOpen/didChange`;
- `applied_version`: последняя версия, реально применённая runtime writer path к semantic snapshot.

Latency-critical orchestration для interactive операций MUST использовать `applied_version` как критерий фактической готовности snapshot. `received_version` MUST NOT считаться эквивалентом готовности semantic состояния.

#### Scenario: Received версия опережает applied версию
- **GIVEN** сервер получил `didChange` до версии `V+1`
- **AND** runtime ещё применил только версию `V`
- **WHEN** интерактивный completion запрошен для `V+1`
- **THEN** состояние рассматривается как "latest ещё не applied"
- **AND** orchestration использует bounded wait/stale policy, а не assumes-ready по received версии

### Requirement: Completion возвращает context implicit symbols в поддерживаемых module contexts (MUST)
Система MUST включать context implicit symbols в non-member completion для поддерживаемых модульных контекстов в соответствии с `ModuleType` и descriptor-based binding contract.

Для `FormModule` минимально MUST быть доступны:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

Для `ManagerModule`/`ObjectModule`/`RecordSetModule` MUST быть доступны соответствующие implicit symbols из контекстной матрицы.

#### Scenario: `FormModule` non-member completion включает implicit symbols
- **GIVEN** курсор находится в модуле формы в поддерживаемом контексте
- **WHEN** IDE запрашивает non-member completion
- **THEN** выдача включает `ЭтотОбъект`, `Объект`, `Форма`, `ЭтаФорма`, `Элементы`, `Параметры`

### Requirement: Member completion для implicit symbols включает свойства и методы (MUST)
Система MUST возвращать в member completion для implicit symbols и свойства, и методы, полученные через descriptor/facet-aware lookup.

Система MUST классифицировать items детерминированно по kind (`property`/`method`) и выполнять case-insensitive дедупликацию по canonical key.
Canonical key MUST включать semantic owner identity и scope, чтобы кандидаты из разных owner-контекстов не схлопывались в один item без явного правила объединения.

#### Scenario: `ЭтотОбъект.` возвращает свойства и методы object facet
- **GIVEN** код в модуле объекта документа использует `ЭтотОбъект.`
- **WHEN** IDE запрашивает member completion
- **THEN** completion включает свойства object facet (например, `Ссылка`)
- **AND** completion включает методы object facet (например, `Записать`)

### Requirement: `FormModule.Объект` completion использует фиксированный provider chain (MUST)
Для `FormModule.Объект` система MUST формировать members в порядке:
1. form shape members,
2. intrinsic supplement (whitelist),
3. applied object facet members,
4. fallback members (если применимо).

Intrinsic supplement MUST быть additive-only и MUST NOT переопределять/удалять members из facet metadata.
Система MUST разделять collection order и precedence policy:
- collection order определяет порядок формирования/показа;
- precedence policy определяет победителя при конфликте одноимённых members.
При конфликте intrinsic vs repository/facet members MUST выигрывать repository/facet member независимо от order обхода.

#### Scenario: `Объект.` в форме документа объединяет shape, intrinsic и facet members
- **GIVEN** модуль формы документа и курсор на `Объект.`
- **WHEN** IDE запрашивает member completion
- **THEN** completion включает реквизиты формы из form shape
- **AND** completion включает гарантированные intrinsic properties (минимум: `Ссылка`, `ПометкаУдаления`)
- **AND** completion включает методы applied object facet (например, `Записать`)

#### Scenario: Конфликт intrinsic и facet member имени не ломает precedence
- **GIVEN** для `FormModule.Объект` существует одноимённый member в intrinsic и repository/facet источнике
- **WHEN** IDE формирует member completion
- **THEN** в выдаче используется repository/facet версия member
- **AND** intrinsic версия не переопределяет repository/facet metadata

### Requirement: Completion для implicit symbols согласован с v2 type snapshot consumers (MUST)
Система MUST использовать тот же owner resolution результат для completion, hover, type-at-position и semantic member validation в рамках одного snapshot/revision.

#### Scenario: Member, предложенный completion, не даёт ложный `NonExistentProperty`
- **GIVEN** completion предложил member для `Объект.`
- **WHEN** пользователь выбирает member и выполняется semantic diagnostics/hover
- **THEN** diagnostics не возвращает ложный `NonExistentProperty` для этого member
- **AND** hover/type-at-position резолвят owner через тот же descriptor/facet контекст

### Requirement: Context-bound implicit symbols не предлагаются в `*БезКонтекста` (MUST)
Система MUST NOT предлагать context-bound implicit symbols в non-member completion внутри процедур/функций `*БезКонтекста`.

#### Scenario: `&НаСервереБезКонтекста` не предлагает `ЭтотОбъект`
- **GIVEN** курсор находится внутри процедуры `&НаСервереБезКонтекста`
- **WHEN** IDE запрашивает non-member completion
- **THEN** completion не содержит context-bound symbols, такие как `ЭтотОбъект` и `Объект`

### Requirement: Completion output остаётся bounded и детерминированным в интерактивном режиме (MUST)
Система MUST ограничивать количество возвращаемых completion items фиксированным limit.
Если после ranking/dedup кандидатов больше лимита, система MUST выставлять `isIncomplete = true`.
При одинаковом snapshot/revision порядок выдачи MUST быть детерминированным.

#### Scenario: Количество кандидатов превышает limit
- **GIVEN** completion контекст, где количество кандидатов превышает системный limit
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает не более limit items
- **AND** `isIncomplete` установлен в `true`

### Requirement: User-facing label policy для FormModule.Объект отделён от owner-facet labels (MUST)
Система MUST использовать для `FormModule.Объект` user-facing label, согласованный с form-data семантикой.

Система MUST NOT отображать `FormModule.Объект` как owner object facet label (`ДокументОбъект.X`, `СправочникОбъект.X`) в compact/full/detailed режимах.

Label policy MUST быть одинаковой между `hover`, `diagnostics`, `completion`, `type-at-position`.

#### Scenario: User-facing label `Объект` согласован с form-data семантикой
- **GIVEN** выражение `Объект` в модуле формы документа
- **WHEN** пользователь запрашивает hover и diagnostics
- **THEN** user-facing type label не использует owner object facet представление
- **AND** вывод согласован между всеми v2 consumers

### Requirement: Applied object modules MUST резолвить bare identifiers через owner-member fallback
Для `ObjectModule` и `RecordSetModule` система MUST резолвить unqualified identifier в следующем порядке:
1. локальная область (параметры, локальные переменные, module vars),
2. глобальный контекст/коллекции/общие модули,
3. explicit common module type,
4. members implicit owner (`ЭтотОбъект`/`Объект`),
5. только затем `UndeclaredVariable`.

Система MUST применять этот контракт единообразно в `type-at-position`, `diagnostics`, `hover`, `completion`.

#### Scenario: Прямой реквизит документа в ObjectModule не считается необъявленным
- **GIVEN** `Documents/<Doc>/Ext/ObjectModule.bsl`
- **WHEN** код вызывает `ЗначениеЗаполнено(ДоговорКонтрагента)` без префикса `ЭтотОбъект.`
- **THEN** `ДоговорКонтрагента` резолвится как member типа `ДокументОбъект.<Doc>`
- **AND** диагностика `Необъявленная переменная` не генерируется

### Requirement: Applied owner fallback MUST NOT ослаблять FormModule strict semantics
Включение owner-member fallback для applied object modules MUST NOT возвращать dual-layer поведение в `FormModule`.

#### Scenario: FormModule остаётся strict form-data при включенном applied fallback
- **GIVEN** owner-member fallback для applied object modules включен
- **WHEN** пользователь запрашивает members для `FormModule.Объект`
- **THEN** выдача строится по strict form-data semantics
- **AND** members из `ДокументОбъект.*` / `СправочникОбъект.*` не подмешиваются

### Requirement: Системные members metadata object MUST быть доступны при прямом обращении
В applied object modules системные members владельца (`ОбменДанными`, `ДополнительныеСвойства` и эквивалентные object-context members) MUST резолвиться через owner-member fallback даже без явного квалификатора.

#### Scenario: DataExchange и AdditionalProperties в обработчике записи набора записей
- **GIVEN** `InformationRegisters/<Reg>/Ext/RecordSetModule.bsl`
- **WHEN** код использует `ОбменДанными.Загрузка` и `ДополнительныеСвойства.Свойство(...)`
- **THEN** оба идентификатора резолвятся как properties объекта набора записей
- **AND** `UndeclaredVariable` diagnostics отсутствует

### Requirement: Exported manager methods MUST резолвиться через metadata collection path
Exported процедуры/функции manager module MUST быть доступны в резолве вызовов вида `КоллекцияМетаданных.<ИмяОбъекта>.<Метод>(...)`.

#### Scenario: RecordSetModule вызывает exported метод manager module регистра
- **GIVEN** `InformationRegisters/<Reg>/Ext/ManagerModule.bsl` содержит `Функция ВладелецБезопасногоХранилища(...) Экспорт`
- **WHEN** код в `RecordSetModule` вызывает `РегистрыСведений.<Reg>.ВладелецБезопасногоХранилища(...)`
- **THEN** метод успешно резолвится как manager member
- **AND** не генерируется `Undefined function/procedure` или `NonExistentMethod`

### Requirement: Manager facet MUST включать predefined members из конфигурации
Система MUST парсить predefined metadata (`Predefined.xml`/`PredefinedDataName`) и добавлять эти элементы как readonly properties manager-фасета для поддерживаемых metadata kinds.

#### Scenario: Предопределенный счет доступен через ПланСчетовМенеджер
- **GIVEN** конфигурация содержит `ChartsOfAccounts/<Chart>/Ext/Predefined.xml` с элементом `ГотоваяПродукция`
- **WHEN** код обращается к `ПланыСчетов.<Chart>.ГотоваяПродукция`
- **THEN** member резолвится как predefined manager property
- **AND** выражение не даёт `Свойство не существует`

### Requirement: Hover/completion ordering MUST быть детерминированным после merge provider-слоёв
После добавления owner-member fallback и predefined manager members система MUST выдавать properties/methods в `hover` и `completion` в стабильном алфавитном порядке.

#### Scenario: Hover для Объект и ЭтотОбъект стабилен по сортировке
- **GIVEN** тип содержит metadata properties, platform members и predefined members
- **WHEN** пользователь запрашивает hover в одном и том же snapshot
- **THEN** порядок properties/methods детерминирован и алфавитный
- **AND** порядок не зависит от внутреннего порядка обхода provider-цепочки

### Requirement: Root-cause drilldown метрики semantic pipeline имеют фиксированную low-cardinality размерность (MUST)
Система MUST публиковать дополнительный stage-level observability слой, позволяющий локализовать latency/regression до комбинации:
- `origin` (минимум: `lsp`, `agent`),
- `operation` (значения из фиксированного `SemanticOperation` enum),
- `stage` (значения из фиксированного `ObservabilityStage` enum),
- `outcome` или `reason` (фиксированный набор).

Drilldown слой MUST оставаться low-cardinality:
- значения MUST браться только из фиксированных enum/классификаторов;
- metric keys MUST NOT включать путь файла, URI, symbol name, свободный пользовательский ввод.

Система MUST предоставлять минимум следующие семейства drilldown-метрик:
- stage totals;
- stage latency histograms;
- cancellation/outcome/reason counters;
- parse/IR skip reason counters.

#### Scenario: Узкое место локализуется до operation+stage+reason
- **GIVEN** в warm-path профиле растет `completion_duration_ms`
- **WHEN** анализируется observability snapshot
- **THEN** по drilldown-метрикам можно однозначно определить проблемную комбинацию `operation+stage`
- **AND** видно, что вклад вызван конкретной `reason` (например, cancellation или skip), а не агрегированным `*_other`

### Requirement: Канонический event model является единственным источником observability semantics (MUST)
Система MUST описывать emission observability через единый канонический event model (transport-agnostic), общий для LSP/web/MCP.

Каноническое событие MUST включать:
- `family`;
- `origin`;
- `value`;
- `operation` и `stage` для stage-семейств.

Контекстные измерения (`outcome`, `reason`, `query_kind`, `work_class`) MAY применяться только там, где это разрешено schema правилом `family`.

Недопустимые сочетания измерений MUST NOT публиковаться как отдельные метрики и MUST фиксироваться контрактным сигналом нарушения schema.

Дополнительно для drift-hardening:
- наборы допустимых `operation/stage/reason` MUST задаваться typed registry (single source of truth);
- canonical normalization и legacy projection MUST строиться из этого же registry;
- добавление нового значения taxonomy без полного mapping MUST детектироваться contract tests до merge.

#### Scenario: Добавление нового stage без registry mapping блокируется валидацией
- **GIVEN** разработчик добавил новый runtime stage в pipeline
- **WHEN** не обновлены typed registry и projection mapping
- **THEN** contract/parity tests падают
- **AND** изменение не может быть принято до восстановления полной deterministic materialization

### Requirement: Dual-write rollout использует единый канонический observability контракт (MUST)
При внедрении drilldown слоя система MUST сохранять backward compatibility fixed-key метрик через dual-write из одного канонического источника событий.

Система MUST соблюдать следующие инварианты:
- канонический контракт задаёт семантику метрик;
- drilldown является primary representation канонического контракта;
- legacy fixed keys являются compatibility-проекцией канонического контракта и MUST NOT иметь отдельную независимую семантику;
- mapping каноника -> fixed keys MUST быть детерминированным и единым для LSP/web/MCP;
- dual-write materialization MUST выполняться в одном centralized projection pipeline (backend-first) в shared runtime;
- adapter-layer MUST NOT публиковать drilldown/legacy метрики напрямую в обход канонического event pipeline.

Дополнительно для precompute observability:
- queue/exec/build для `type_index_precompute` MUST иметь dedicated projection keys;
- эти события MUST NOT сворачиваться в `runtime_other_*` для legacy/canonical представлений;
- projection completeness MUST проверяться контрактным тестом.

#### Scenario: Type-index precompute queue/exec не смешивается с `other`
- **GIVEN** runtime публикует canonical события `type_index_precompute` queue/exec/build
- **WHEN** выполняется dual-write materialization
- **THEN** увеличиваются только dedicated precompute ключи
- **AND** `runtime_other_*` не получает вклад этих событий

### Requirement: Runtime saturation и singleflight effectiveness наблюдаемы отдельным слоем (MUST)
Система MUST публиковать observability-метрики, которые отделяют queue/CPU contention от логики semantic стадий.

Обязательные группы saturation/effectiveness метрик:
- waiters/permits/queue-depth для runtime budget/очередей;
- singleflight effectiveness по `query_kind` (leader/shared);
- сигнал о невозможности построить singleflight key (`key_unavailable`).

Все значения MUST быть low-cardinality и пригодны для агрегирования между интерфейсами.

#### Scenario: Queue contention различим от проблем semantic query
- **GIVEN** наблюдается рост `runtime_queue_wait` latency
- **WHEN** анализируется saturation/effectiveness слой
- **THEN** можно определить, вызван ли рост нехваткой runtime budget (waiters/permits/queue depth)
- **AND** можно оценить, помог ли singleflight (`shared`) или не сработал из-за `key_unavailable`

### Requirement: didChange-path diagnostics ограничен дешёвыми инкрементальными шагами (MUST)
Система MUST обрабатывать `textDocument/didChange` через fast-path, который не запускает полный тяжёлый semantic пересчёт на каждый символ.

Для fast-path на `didChange`:
- MUST выполняться только дешёвые и инкрементальные шаги оркестрации (применение версии/состояния, минимальные локальные проверки);
- MUST NOT выполняться тяжёлые стадии полного diagnostics-пайплайна;
- MUST сохраняться совместимость с существующим strict-latest publish контрактом (см. требование про diagnostics publish latest-version).

#### Scenario: Частое редактирование не запускает полный heavy diagnostics на каждый символ
- **GIVEN** пользователь быстро вводит текст и генерирует серию `didChange` (`V`, `V+1`, `V+2`, ...)
- **WHEN** LSP обрабатывает входящие события
- **THEN** на каждом событии выполняется только fast-path
- **AND** тяжёлые проверки не запускаются синхронно на каждый `didChange`

### Requirement: Тяжёлые diagnostics стадии выполняются deferred с debounce и background class (MUST)
Система MUST выполнять полный тяжёлый diagnostics путь в deferred-профиле:
- запуск через debounce для coalescing серий `didChange`;
- выполнение в background CPU class;
- новая версия документа MUST supersede устаревший deferred запуск.

Система MUST проверять актуальность версии/поколения перед каждой дорогой стадией и прекращать устаревшую задачу до publish.

#### Scenario: Более новая версия supersede устаревший deferred запуск
- **GIVEN** запущен deferred diagnostics для версии `V`
- **AND** до завершения приходит `didChange` для версии `V+1`
- **WHEN** deferred задача для `V` доходит до следующей тяжёлой стадии
- **THEN** задача для `V` завершается как устаревшая (`superseded`) без публикации
- **AND** система продолжает обработку только актуального запуска

### Requirement: Diagnostics publish проверяет revision token с generation (MUST)
Публикация diagnostics MUST выполняться только при совпадении актуального revision token:
- `file_version`;
- `deps_id`;
- `settings_id`;
- `diagnostics_generation` (или эквивалентного monotonic токена запуска).

Результат для устаревшего token MUST NOT публиковаться и MUST NOT перезаписывать более новый publish.

#### Scenario: Устаревший запуск не может перезаписать актуальные diagnostics
- **GIVEN** heavy diagnostics запуск для поколения `G` и версии `V`
- **AND** затем пришла новая версия, создавшая поколение `G+1`
- **WHEN** запуск `G` завершается позже
- **THEN** publish для `G` отклоняется
- **AND** опубликованным остаётся только результат для актуального поколения

### Requirement: Дорогие проверки запускаются только по didSave и/или idle trigger (MUST)
Система MUST отделять expensive проверки от fast `didChange` пути.

Expensive-проверки MUST запускаться:
- по `textDocument/didSave`, если событие доступно;
- либо по `idle` trigger после отсутствия новых `didChange` в течение конфигурируемого окна.

Эти проверки MUST NOT быть обязательной частью каждого `didChange` запуска.

Если expensive diagnostics запускаются по `didSave`, система MAY делать bounded first-publish
fastlane до final heavy publish, но такой fastlane:
- MUST оставаться same-version truthful для сохранённой revision;
- MUST NOT публиковать older-version diagnostics;
- MUST NOT ждать unbounded `wait_for_file_version` только ради final heavy completeness.

#### Scenario: Heavy-проверки выполняются после паузы или сохранения
- **GIVEN** пользователь печатает без сохранения
- **WHEN** идут последовательные `didChange`
- **THEN** heavy-проверки не выполняются на каждый символ
- **AND** heavy-проверки запускаются только после `didSave` или достижения `idle` окна

#### Scenario: didSave first publish bounded, even if writer apply lags
- **GIVEN** пользователь сохранил документ на версии `V`
- **AND** analysis writer ещё не догнал `V` в applied revision state
- **WHEN** запускается diagnostics path для `didSave`
- **THEN** система делает bounded first publish для версии `V` без seconds-scale ожидания `wait_for_file_version`
- **AND** первый publish использует только same-version truthful artifacts
- **AND** final heavy publish для `V` может завершиться вторым проходом позже

### Requirement: Observability фиксирует diagnostics trigger/profile/supersede причины (MUST)
Канонический observability контракт MUST фиксировать diagnostics pipeline по low-cardinality измерениям:
- `trigger` (`did_change|did_open|did_save|idle`);
- `profile` (`fast|debounced_full|save_fastlane|idle_heavy`);
- `reason` (`published|superseded_version|superseded_generation|cancelled` минимум).

Dual-write MUST оставаться детерминированным из канонического event model: drilldown как primary, legacy как projection.

Дополнительно для `syntax_diagnostics` stage:
- канонический observability contract MUST включать low-cardinality измерение `mode`, показывающее parse mode, использованный для текущей ревизии syntax diagnostics;
- поле `mode` MUST интерпретироваться stage-aware:
  - для `syntax_diagnostics` `mode` означает parse mode;
  - для completion-related stages `mode` сохраняет completion-routing semantics существующего контракта;
- schema validation / typed registry MUST запрещать недопустимые сочетания stage/mode;
- допустимые значения `mode` MUST быть ограничены `incremental|reused|full|other`;
- для diagnostics path без version-bound `ParseSnapshot` (включая `non-LSP` origins, если shared parse snapshot отсутствует) система MUST публиковать `mode=other`;
- `full` MUST использоваться только когда shared parse snapshot / parse-report contract для текущей ревизии явно указывает на full parse path;
- mode-aware latency MUST позволять сравнить syntax diagnostics stage между parse mode без high-cardinality labels;
- legacy fixed-key метрика `intellisense_v2_syntax_diagnostics_query_ms` MUST сохраняться как aggregate compatibility projection и MUST NOT терять backward compatibility.

Для save-triggered first publish observability MUST отдельно позволять доказать:
- latency до первого publish после `didSave`;
- был ли использован `save_fastlane` или только final heavy path;
- не ушла ли задержка в `wait_for_file_version`/apply lag до first publish.

#### Scenario: Метрики показывают latency syntax diagnostics по parse mode
- **GIVEN** mixed нагрузка, где syntax diagnostics в одних ревизиях использует `incremental` или `reused`, а в других `full`
- **WHEN** запрашивается observability snapshot
- **THEN** канонический observability contract содержит mode-aware latency разрез для `syntax_diagnostics`
- **AND** значения `mode` ограничены `incremental|reused|full|other`
- **AND** aggregate legacy метрика `intellisense_v2_syntax_diagnostics_query_ms` остаётся доступной

#### Scenario: Non-LSP path без shared parse snapshot деградирует в `other`
- **GIVEN** diagnostics выполняется через origin/path, где для текущей ревизии нет version-bound `ParseSnapshot`
- **WHEN** публикуется observability snapshot для `syntax_diagnostics`
- **THEN** канонический observability contract использует `mode=other`
- **AND** система MUST NOT синтезировать `incremental`, `reused` или `full` из adapter-local предположений

#### Scenario: Save fastlane distinguishable from heavy follow-up
- **GIVEN** first diagnostics refresh после `didSave` выполняется через bounded fastlane
- **WHEN** анализируется observability snapshot или checked-in acceptance report
- **THEN** first publish помечается отдельным profile `save_fastlane`
- **AND** heavy follow-up остаётся различимым как `idle_heavy`
- **AND** evidence позволяет отличить fastlane успех от apply-lag wait regression

### Requirement: Completion v2 учитывает trigger context LSP и сохраняет parity между trigger modes (MUST)
Система MUST использовать `CompletionParams.context` (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`) как часть completion policy.

Для одной и той же ревизии документа и позиции member-access:
- completion по `TriggerCharacter='.'` MUST использовать тот же semantic контекст owner/member resolution, что и `Invoked`;
- `TriggerCharacter='.'` MUST NOT деградировать в нерелевантный keyword-only ответ только из-за trigger mode;
- различия между trigger modes допускаются только в пределах explicit degraded semantics (`isIncomplete=true`).

#### Scenario: `TriggerCharacter='.'` и `Invoked` дают согласованный member-access контекст
- **GIVEN** курсор стоит в позиции после `expr.`
- **AND** текст/ревизия документа не менялись между запросами
- **WHEN** IDE запрашивает completion сначала как `TriggerCharacter='.'`, затем как `Invoked`
- **THEN** оба ответа используют согласованный receiver/member semantic контекст
- **AND** ответ по `TriggerCharacter='.'` не сводится к нерелевантной keyword-only выдаче

### Requirement: Интерактивный completion v2 имеет bounded latency и полезную деградацию (MUST)
Система MUST обеспечивать bounded completion response при typing-load (`didChange` bursts) и MUST NOT зависать из-за ожидания консистентности ревизий.

Если fresh semantic данные временно недоступны, система MUST возвращать полезный degraded completion для распознанного member-access контекста с `isIncomplete=true`, вместо terminal-empty ответа, вызванного только transient недоступностью IR.

Система MUST сохранять observability по стадиям latency/fallback, достаточную для регрессионного контроля интерактивного пути, включая:
- разрез по trigger mode (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`, `None`),
- parity drift индикаторы между `TriggerCharacter='.'` и `Invoked`,
- счётчики transient member-access terminal-empty и fallback_unavailable.

#### Scenario: Под серией `didChange` completion остаётся bounded и не даёт transient terminal-empty
- **GIVEN** пользователь быстро печатает, и IDE отправляет серию `didChange`
- **WHEN** IDE запрашивает completion в member-access контексте во время transient нагрузки
- **THEN** completion возвращается в bounded интерактивном времени (без зависания)
- **AND** при временной недоступности fresh IR сервер возвращает degraded `isIncomplete=true` ответ вместо terminal-empty результата, если контекст позволяет полезные candidates

#### Scenario: Trigger-aware observability доступна для контроля parity
- **GIVEN** IDE выполняет completion запросы в режимах `TriggerCharacter='.'`, `Invoked` и `TriggerForIncompleteCompletions`
- **WHEN** система публикует observability метрики completion
- **THEN** метрики содержат trigger mode разрез и позволяют сравнить parity между `.` и `Invoked`
- **AND** transient member-access terminal-empty и fallback_unavailable отражаются отдельными счётчиками

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

### Requirement: Scale-aware diagnostics policy защищает интерактивный путь на больших модулях при churn (MUST)
Система MUST определять состояние `large + churn` для текущего документа и в этом состоянии MUST переключать diagnostics orchestration в интерактивно-безопасный режим.

Для состояния `large + churn`:
- `textDocument/didChange` MUST выполнять только fast diagnostics path;
- тяжелые diagnostics стадии (`debounced_full`, `idle_heavy`) MUST NOT запускаться синхронно на каждый `didChange`;
- тяжелые стадии MUST запускаться только по `idle` и/или `didSave` trigger;
- strict latest-version publish инварианты для diagnostics MUST сохраняться.

#### Scenario: Heavy diagnostics не конкурирует с completion на каждый символ в `large + churn`
- **GIVEN** открыт большой модуль, и IDE генерирует burst `didChange`
- **WHEN** система классифицирует состояние как `large + churn`
- **THEN** на `didChange` выполняется только fast path
- **AND** heavy diagnostics переносится на `idle`/`didSave`
- **AND** интерактивный completion обслуживается без синхронного ожидания heavy path

### Requirement: Runtime scheduling имеет явный интерактивный приоритет с fairness для background (MUST)
Система MUST обслуживать интерактивные операции (`completion`, `hover`, `signatureHelp`) с приоритетом относительно background diagnostics задач в runtime очередях.

Система MUST одновременно обеспечивать fairness:
- background diagnostics MUST получать гарантированный прогресс;
- интерактивный приоритет MUST NOT приводить к бесконечному starvation background diagnostics.

#### Scenario: Интерактивный запрос не блокируется backlog background задач
- **GIVEN** в runtime очереди накоплен background diagnostics backlog
- **WHEN** приходит интерактивный completion запрос
- **THEN** интерактивный запрос обслуживается с приоритетом
- **AND** background backlog продолжает выполняться по fairness-правилам

### Requirement: Observability отражает policy-переходы `large + churn` и причины deferred heavy-path (MUST)
Система MUST публиковать low-cardinality observability сигналы для scale-aware policy:
- факт входа/выхода из `large + churn`;
- причины отложенного heavy diagnostics запуска;
- связь policy-переходов с stage-level latency completion пути.

#### Scenario: Root-cause задержки completion локализуется через policy и stage метрики
- **GIVEN** растет latency интерактивного completion на большом модуле
- **WHEN** анализируется observability snapshot
- **THEN** видны события policy-переходов `large + churn`
- **AND** по stage-level метрикам можно отделить queue contention от query bottleneck

### Requirement: Parse snapshot v2 является version-bound и общим для completion/diagnostics (MUST)
Система MUST поддерживать явный `ParseSnapshot` на уровне файла, связанный с конкретной версией документа.

`ParseSnapshot` MUST включать как минимум:
- версию файла, для которой вычислен snapshot;
- parse result и line index;
- дерево parser backend, пригодное для incremental update;
- changed ranges (если snapshot был получен инкрементально).

Completion и diagnostics MUST использовать один и тот же snapshot контракт для одной и той же ревизии.

#### Scenario: Completion и diagnostics читают согласованный parse state
- **GIVEN** документ версии `V` уже применен в pipeline
- **WHEN** параллельно запрашиваются completion и diagnostics для `V`
- **THEN** обе операции используют общий `ParseSnapshot(V)`
- **AND** система не создает независимые несовместимые parse состояния для одного `file_id/version`

### Requirement: didChange path использует incremental parse с fail-safe full fallback (MUST)
На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и edit chain.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить на full parse для той же версии и фиксировать причину fallback в observability.

#### Scenario: Incremental update fallback не ломает корректность
- **GIVEN** edit chain для `didChange` не может быть применена к текущему дереву
- **WHEN** система обновляет parse state
- **THEN** выполняется full parse fallback для актуальной версии
- **AND** итоговый snapshot остается version-consistent
- **AND** причина fallback доступна в observability

### Requirement: Changed-ranges используются для ограниченного downstream recompute (MUST)
Система MUST передавать changed-range информацию в downstream стадии и ограничивать пересчет затронутыми диапазонами, когда это не нарушает корректность.

Если range-ограничение не может гарантировать корректный результат, система MUST выполнить полный пересчет соответствующей стадии.

#### Scenario: Burst мелких правок не вызывает полный тяжелый пересчет каждый раз
- **GIVEN** пользователь вносит серию локальных правок в небольшой диапазон большого модуля
- **WHEN** pipeline обрабатывает последовательные ревизии
- **THEN** затронутые стадии используют changed ranges для локального пересчета
- **AND** полный пересчет выполняется только при невозможности безопасного range-режима

### Requirement: Completion under large-module churn использует bounded wait и fail-closed current-revision path (MUST)
Для интерактивного completion на больших модулях в состоянии churn система MUST использовать только exact current-revision precomputed artifact (`serve-only`) или явный fail-closed miss для текущей revision.

Completion under churn MUST NOT блокироваться секундными хвостами ожидания latest-path.
Интерактивный request path MUST NOT запускать sync parse/index compute, даже если exact artifact еще недоступен.
Completion under churn MUST NOT исчерпывать bounded wait на фазе `wait_for_file_version` только потому, что latest same-file apply после document-sync handoff остаётся в очереди позади slow background work.
Completion under churn MUST NOT завершаться `exact_deadline`, если `observed_file_version` уже достиг requested current revision, но `CompletionHeadArtifact` всё ещё отсутствует только потому, что head publish сериализован позади `ExactSemanticArtifact`, `type_index_precompute` или deferred diagnostics.

Для этого change и `prepare_timeout@wait_for_file_version`, и post-apply `head_ready=false` `exact_deadline` считаются regressions current-revision readiness fast lane, а не допустимым bounded fail-closed поведением.

#### Scenario: Under churn completion отдаёт bounded fail-closed ответ без sync parse/index
- **GIVEN** большой модуль находится в активном churn режиме
- **AND** exact latest artifact временно недоступен в пределах wait budget
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает bounded fail-closed response для текущей revision
- **AND** sync parse/index compute не выполняется в интерактивном request path

#### Scenario: Post-handoff apply backlog считается regression, а не acceptable miss
- **GIVEN** `didChange` уже зарегистрировал handoff для requested revision `V`
- **AND** completion запрашивается для той же revision `V`
- **WHEN** bounded wait истекает на фазе `wait_for_file_version`, потому что latest same-file apply всё ещё стоит позади background backlog
- **THEN** такой исход считается regression readiness scheduler
- **AND** не считается допустимым fail-closed поведением under churn

#### Scenario: Post-apply отсутствие head считается regression, а не normal exact latency
- **GIVEN** completion уже наблюдает `observed_file_version >= requested current revision`
- **AND** `head_ready=false`, потому что publish `CompletionHeadArtifact` ждёт exact/type-index/deferred diagnostics path
- **WHEN** completion завершает exact wait по deadline
- **THEN** такой исход считается regression head-readiness fast lane
- **AND** не считается допустимой exact-upgrade latency

### Requirement: После fail-closed miss система выполняет асинхронный latest refresh без user-facing блокировки (MUST)
После bounded fail-closed completion miss система MUST продолжать или запускать background refresh latest snapshot.

Fail-closed miss MUST завершаться быстро в пределах bounded policy без длительного блокирования пользователя.

#### Scenario: Fail-closed miss запускает или продолжает background refresh
- **GIVEN** completion завершился fail-closed из-за недоступности exact latest artifact
- **WHEN** пользователь продолжает работу
- **THEN** latest refresh выполняется асинхронно в фоне
- **AND** последующие completion запросы могут перейти на latest без блокирующего ожидания предыдущего refresh

### Requirement: Quality gate оценивает churn-aware completion отдельно от non-churn baseline (MUST)
Scale-aware gate MUST публиковать отдельные pass/fail оценки для churn-aware профиля и non-churn baseline.

Gate MUST включать как минимум:
- latency метрики (`completion_duration_ms`, stage-level breakdown);
- fail-closed counters (`fallback_unavailable`, `wait_budget_exhausted`);
- stale guardrail counters (`stale_served`, `stale_fallback`), которые на authoritative fixtures MUST оставаться нулевыми;
- sample sufficiency для warm фазы.

#### Scenario: Churn regression выявляется независимо от non-churn профиля
- **GIVEN** non-churn профиль проходит по latency
- **AND** churn-aware профиль деградирует
- **WHEN** выполняется scale-aware gate
- **THEN** gate явно помечает провал churn-aware части
- **AND** отчет содержит stage-level root-cause данные для churn профиля

### Requirement: Superseded diagnostics задачи отменяются до завершения heavy path (MUST)
При появлении более новой ревизии документа система MUST помечать соответствующие in-flight diagnostics задачи как superseded и инициировать их отмену до завершения тяжелых стадий.

`DebouncedFull` и `IdleHeavy` профили MUST поддерживать supersession cancellation.

#### Scenario: Burst didChange отменяет устаревшие heavy diagnostics
- **GIVEN** для файла запущена heavy diagnostics задача на ревизии `R`
- **AND** приходит более новая ревизия `R+1`
- **WHEN** scheduler пересчитывает очередность задач
- **THEN** задача `R` переводится в superseded cancellation
- **AND** heavy стадии для `R` не продолжаются до полного завершения, если достигнут cancel checkpoint

### Requirement: Cancellation checkpoints обязательны между heavy diagnostics стадиями (MUST)
Система MUST иметь кооперативные cancellation checkpoints как минимум:
- перед запуском parse/syntax heavy стадии;
- между syntax и semantic heavy стадиями;
- перед publish diagnostics.

Задача, получившая superseded cancel, MUST завершаться без publish.

#### Scenario: Superseded задача не публикует diagnostics
- **GIVEN** heavy diagnostics для ревизии `V` уже вычислена частично
- **AND** до publish приходит ревизия `V+1`
- **WHEN** выполняется финальный checkpoint перед publish
- **THEN** результат `V` не публикуется
- **AND** publish выполняется только для актуальной ревизии

### Requirement: Observability различает superseded cancellation и прочие причины cancel (MUST)
Система MUST публиковать low-cardinality signals для diagnostics cancellation с фиксированными причинами:
- `superseded_generation`
- `superseded_version`
- `client_cancel`
- `other_cancel`

#### Scenario: Root-cause cancel виден в метриках
- **GIVEN** под churn устаревшие diagnostics регулярно отменяются
- **WHEN** анализируется observability snapshot
- **THEN** в метриках видны отмены по `superseded_generation`/`superseded_version`
- **AND** они не смешиваются с `client_cancel`

### Requirement: Scale-aware baseline artifact для completion latency является обязательным и versioned (MUST)
Система MUST сохранять и использовать versioned baseline artifact для latency gate completion v2.

Baseline artifact MUST включать:
- профили `large` и `small`;
- фазы `start`, `cold`, `warm`;
- минимум следующие метрики для completion-контура:
  - `completion_duration_ms`;
  - `intellisense_v2_wait_for_file_version_completion_ms`;
  - `intellisense_v2_snapshot_completion_ms`;
  - `intellisense_v2_ir_query_completion_ms`;
- sample size (`n`) для каждой фазы/метрики;
- явный `pass/fail` summary по gate-критериям.

Gate MUST падать, если baseline artifact отсутствует, повреждён или не содержит обязательных полей.

#### Scenario: Gate использует baseline artifact и даёт воспроизводимый verdict
- **GIVEN** baseline artifact присутствует и валиден
- **WHEN** выполняется scale-aware perf прогон
- **THEN** система вычисляет ratio/threshold verdict детерминированно из baseline и текущих метрик
- **AND** итоговый отчёт содержит `pass/fail` и все обязательные поля

#### Scenario: Отсутствующий baseline блокирует принятие результата
- **GIVEN** baseline artifact отсутствует или невалиден
- **WHEN** запускается quality gate
- **THEN** gate завершается ошибкой конфигурации
- **AND** прогон не считается валидным доказательством ускорения

### Requirement: Completion v2 и observability completion имеют versioned contract baseline (MUST)
Система MUST поддерживать versioned contract baseline для интерактивного completion v2 в `contracts/**`.

Baseline MUST покрывать как минимум:
- completion surface: trigger context semantics (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`, `None`) и outcome классы (`ok_non_empty`, `ok_empty`, `fallback_unavailable`);
- observability surface: trigger mode метрики, parity drift, member-access terminal-empty, fail-closed счётчики и bounded `type_index` reason taxonomy без stale/degraded public labels.

#### Scenario: Изменение completion semantics требует обновления contract baseline
- **GIVEN** разработчик меняет semantics интерактивного completion v2 или имена/лейблы связанных метрик
- **WHEN** change проходит ревью
- **THEN** соответствующий versioned contract baseline в `contracts/**` обновлён
- **AND** для breaking изменения выполнен major version bump по policy

### Requirement: Event-driven precompute `type_index` выполняется на `didOpen/didChange` и публикует version-bound artifacts (MUST)
Система MUST вычислять `type_index` как precompute-артефакт при обработке событий документа (`didOpen`, `didChange`) и MUST связывать результат с ключом:
- `file_id`
- `file_version`
- `deps_id`
- `settings_id`

Система MUST гарантировать latest-wins для одного `file_id`: артефакты superseded версий не должны становиться источником latest serving.

#### Scenario: Burst `didChange` публикует только актуальный precompute artifact
- **GIVEN** для одного файла приходят версии `V`, `V+1`, `V+2` в коротком окне
- **WHEN** precompute jobs выполняются конкурентно/асинхронно
- **THEN** serving latest использует только artifact для актуальной версии
- **AND** artifacts superseded версий не публикуются как latest результат

### Requirement: Интерактивный type lookup использует serve-only cache и не запускает sync parse/index (MUST)
Интерактивные запросы (`completion`, `hover`, `signatureHelp`) MUST получать type lookup данные только из precomputed artifact cache.

В request path MUST NOT запускаться синхронный тяжелый compute `parse_result/type_index` для получения ответа пользователю.

При cache miss система MUST завершать запрос в bounded времени через fail-closed outcome (`fallback_unavailable`) и MUST NOT блокировать ответ длительным пересчетом.

#### Scenario: Cache miss не запускает sync тяжелый compute в интерактивном запросе
- **GIVEN** интерактивный completion запрос пришел до готовности exact artifact
- **WHEN** request path обрабатывает запрос
- **THEN** ответ возвращается в bounded времени с fail-closed outcome для текущей revision
- **AND** sync parse/index compute для этого запроса не выполняется

### Requirement: Invalidation артефактов `type_index` детерминирован по `deps_id/settings_id` (MUST)
Система MUST считать artifact невалидным для serving, если изменился `deps_id` или `settings_id` относительно ключа артефакта, даже при совпадении `file_id/file_version`.

#### Scenario: Смена deps invalidates artifact для той же версии файла
- **GIVEN** artifact построен для `(file_id=F, version=V, deps=D1, settings=S1)`
- **AND** runtime переключился на `deps=D2`
- **WHEN** приходит интерактивный запрос для `(F, V)`
- **THEN** artifact `(D1, S1)` не используется как exact latest
- **AND** система отвечает fail-closed для нового ключа, пока exact artifact `(D2, S1)` не станет доступен

### Requirement: Observability контур различает precompute и serve outcomes low-cardinality метками (MUST)
Система MUST публиковать low-cardinality observability метрики отдельно для:
- precompute queue wait/exec/build;
- serving outcomes (`exact`, `fallback_unavailable`);
- anti-rescue guard counters (`stale_served`, `stale_fallback`), которые MUST оставаться нулевыми на authoritative fixtures;
- supersede/cancel причин precompute jobs.

#### Scenario: Root-cause latency виден как precompute lag vs serve miss
- **GIVEN** растет tail latency completion на большом модуле
- **WHEN** анализируется observability snapshot
- **THEN** можно отделить precompute queue lag от serving cache miss path
- **AND** outcome классы serving доступны как low-cardinality counters

### Requirement: Interactive completion v2 имеет обязательные resource budgets по alloc/lock alongside latency (MUST)
Система MUST расширить quality gate интерактивного completion v2: помимо latency, gate MUST учитывать ресурсные бюджеты для warm-path.

Минимальный набор обязательных resource budget метрик:
- `allocations_per_completion`;
- `allocated_bytes_per_completion`;
- `lock_wait_ms_per_completion`;
- `lock_contention_events_per_completion`.

Указанные metric keys MUST использоваться в schema contract без замены на альтернативные имена в пределах одной major версии контракта.
Бюджеты MUST быть versioned в baseline artifact и проверяться на профилях минимум `small`, `large`, `churn`.
Latency часть completion gate MUST проверяться одновременно по относительным порогам к baseline и по абсолютным ceiling budget (`p95/p99`) для warm-path.

#### Scenario: Latency gate проходит, resource gate блокирует регрессию
- **GIVEN** warm latency completion укладывается в целевой SLO
- **WHEN** сравниваются resource metrics с versioned baseline
- **THEN** gate завершается fail, если lock wait или allocations превышают budget
- **AND** change не считается perf-safe

#### Scenario: Relative latency stable, absolute warm-path ceiling exceeded
- **GIVEN** `ratio_p95` и `ratio_p99` к baseline в допустимых пределах
- **AND** абсолютный warm-path `p95` или `p99` превышает утвержденный ceiling
- **WHEN** выполняется completion quality gate
- **THEN** gate завершается fail, даже если relative ratio проходит
- **AND** отчёт фиксирует нарушение абсолютного latency budget как блокирующее

#### Scenario: Отсутствует обязательный canonical metric key
- **GIVEN** perf report не содержит один из обязательных keys (`allocations_per_completion`, `allocated_bytes_per_completion`, `lock_wait_ms_per_completion`, `lock_contention_events_per_completion`)
- **WHEN** evaluator module выполняет completion quality gate
- **THEN** gate завершается fail с причиной `missing_required_metric_field`
- **AND** отчёт не считается валидным input для verdict

### Requirement: Completion observability публикует low-cardinality allocator/lock pressure signals (MUST)
Система MUST публиковать low-cardinality observability для root-cause анализа resource regressions completion пути.

Observability контракт MUST включать как минимум:
- отдельные метрики/поля для allocation pressure и lock contention;
- фиксированные low-cardinality reason labels (например, `allocator_pressure`, `lock_wait`, `queue_backpressure`, `other`);
- связь resource signals со stage-level completion latency для причинно-следственного drilldown.

#### Scenario: Root-cause деградации локализуется до resource класса
- **GIVEN** зафиксирован рост интерактивной completion latency на churn нагрузке
- **WHEN** анализируется observability snapshot
- **THEN** система позволяет различить allocation pressure и lock contention как отдельные причины
- **AND** причина сопоставляется со stage-level latency без high-cardinality шума

### Requirement: Warm interactive completion избегает process-global lock bottleneck в steady-state (MUST)
Система MUST гарантировать, что warm-path интерактивного completion в steady-state не зависит от process-global lock как обязательной точки сериализации каждого запроса.

Если fallback путь временно использует глобальную сериализацию, это MUST быть:
- явно ограничено редкими условиями;
- отражено в observability как отдельная причина деградации;
- покрыто планом устранения в рамках approved ADR.

#### Scenario: Burst completion не упирается в глобальный lock
- **GIVEN** серия `didChange` и параллельных completion запросов на разных файлах
- **WHEN** система работает в warm steady-state режиме
- **THEN** запросы не сериализуются через process-global lock на каждом completion
- **AND** наблюдаемость не показывает устойчивый global-lock bottleneck как нормальный путь

### Requirement: Completion perf verdict вычисляется только Option B evaluator модулем (MUST)
Система MUST использовать dedicated perf-gate evaluator module как единственный источник perf-verdict для интерактивного completion v2.

Нормативные требования:
- все проверяющие контуры (`intellisense_perf` harness, CI gate, runtime acceptance checks) MUST использовать один и тот же evaluator API;
- evaluator MUST читать versioned schema contract `contracts/intellisense-perf-gate/vN/**` и возвращать детерминированный `report`;
- `report` MUST включать единый набор `reason_codes` для latency/resource нарушений и `contract_version`;
- потребители MUST NOT вычислять собственный альтернативный verdict по тем же метрикам вне evaluator module.

#### Scenario: Один и тот же input даёт одинаковый verdict во всех контурах
- **GIVEN** фиксированный набор метрик completion для профилей `small`, `large`, `churn`
- **WHEN** этот набор проверяется через CI gate и локальный harness
- **THEN** оба контура получают одинаковый verdict и reason-codes из одного evaluator module
- **AND** результаты совместимы по `contract_version`

#### Scenario: Schema version mismatch обрабатывается fail-closed
- **GIVEN** consumer передаёт baseline/report с неподдерживаемой версией schema contract
- **WHEN** evaluator module выполняет проверку
- **THEN** gate завершается fail с причиной `unsupported_contract_version`
- **AND** completion change не считается perf-safe до согласованной миграции контракта

### Requirement: Retention policy для `TypeIndexArtifact` детерминирован и count-based (MUST)
Система MUST трактовать retention policy для `TypeIndexArtifact` как count-based контракт:
- `max_versions_per_file_identity` задаёт точное количество хранимых версий на `(file_id, deps_id, settings_id)`;
- версия окна MUST определяться как "latest N", а не через неявную version-gap эвристику;
- eviction MUST быть детерминированным и observability-visible по reason-code taxonomy.

Global guard eviction MUST NOT удалять актуальный exact artifact latest key для текущего `(file_id, version, deps_id, settings_id)`.

#### Scenario: Version window сохраняет только latest N и защищает latest exact
- **GIVEN** для одного `(file_id, deps_id, settings_id)` построены artifacts версий `V1..V4`
- **AND** configured `max_versions_per_file_identity = 2`
- **WHEN** применяется retention + global guard
- **THEN** в окне остаются только `V4` и `V3`
- **AND** актуальный exact artifact latest key не удаляется

### Requirement: Serve-only `type_index` outcomes публикуются единообразно для всех interactive операций (MUST)
Для интерактивных операций, использующих serve-only type lookup (`completion`, `hover`, `signatureHelp`, `definition`), система MUST публиковать `type_index` serve outcome reason из bounded taxonomy:
- `type_index_exact_hit`
- `type_index_fallback_unavailable`

Unknown reason labels MUST быть сведены в `other` и сопровождаться контрактным сигналом нарушения, без увеличения cardinality.

#### Scenario: Hover cache miss фиксируется как `type_index_fallback_unavailable`
- **GIVEN** hover запрошен до готовности exact artifact
- **WHEN** serve-only path завершает запрос без on-demand compute
- **THEN** публикуется reason `type_index_fallback_unavailable`
- **AND** reason учитывается в том же low-cardinality контракте, что и completion/signatureHelp/definition

### Requirement: Perf-gate artifacts должны быть traceable к активному `change_id` (MUST)
Perf reports и gate summaries MUST включать `change_id`, полученный из invocation context текущего прогона.
Источник `expected_change_id` в invocation context MUST иметь фиксированный приоритет:
1. `--change-id` CLI argument;
2. `OPENSPEC_CHANGE_ID` environment variable.

Hardcoded foreign `change_id` в runtime/perf path MUST NOT использоваться.

Правила валидации provenance:
- если invocation context содержит `expected_change_id`, то missing/mismatch/invalid `change_id` в report MUST приводить к fail-fast validation результата (invalid evidence);
- если invocation context НЕ содержит `expected_change_id` (legacy-local режим), отсутствие provenance-полей MAY быть допустимо только для локальной диагностики и MUST NOT использоваться как cutover evidence.

#### Scenario: Mismatch `change_id` блокирует принятие perf evidence
- **GIVEN** perf прогон выполняется для change `X`
- **WHEN** сформированный report содержит другой `change_id` или не содержит его
- **THEN** quality-gate validation завершает прогон как invalid evidence
- **AND** артефакт не используется как доказательство прохождения gate

#### Scenario: Отсутствие expected change-id помечает evidence как неавторитетный
- **GIVEN** perf прогон запущен без `--change-id` и без `OPENSPEC_CHANGE_ID`
- **WHEN** формируется report без provenance `change_id`
- **THEN** локальный прогон не падает только из-за отсутствия provenance
- **AND** такой артефакт не может быть использован как cutover evidence

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `24`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v21`, синхронизированный с текущим authoritative payload и его bounded field-set.

`v24` MUST сохранять additive `v23` ingress/query-body/flush-aware/output-egress semantics, включая grouped `query_bundle*` taxonomy, `response_sent_at_ms`, existing `response_output_*` milestones и `response_flush_completed_at_ms`.

Контракт `v24` MUST включать:

- `version` (числовой номер контракта);
- `traces` (массив completion trace записей).

Каждый trace MUST включать:

- `trace_id`, `request_id`, `uri`, `trigger_mode`;
- `outcome`, `started_at_ms`, `total_duration_ms`;
- `dominant_stage`;
- `prepare_details`;
- `turn_attribution`;
- optional `server_edge_details`;
- `stages`.

Если `server_edge_details` присутствует, additive `v24` post-handler handoff split MAY включать:

- `response_output_handoff_started_at_ms`;
- `response_output_handoff_enqueued_at_ms`;
- `response_ready_to_output_handoff_wait_ms`;
- `response_output_handoff_send_wait_ms`;
- `response_output_handoff_to_writer_wait_ms`.

Если `response_output_handoff_started_at_ms` присутствует, payload MUST включать и `response_output_handoff_enqueued_at_ms`.

Если `response_output_handoff_started_at_ms` присутствует, payload MUST сохранять `response_output_enqueue_completed_at_ms` как legacy compatibility boundary output-writer selection для completion response и MUST включать все три derived fields:

- `response_ready_to_output_handoff_wait_ms`;
- `response_output_handoff_send_wait_ms`;
- `response_output_handoff_to_writer_wait_ms`.

Если `response_ready_to_output_handoff_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_sent_at_ms` и `response_output_handoff_started_at_ms` и MUST NOT включать blocking внутри outbound handoff path.

Если `response_output_handoff_send_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_output_handoff_started_at_ms` и `response_output_handoff_enqueued_at_ms` и MUST NOT включать wait после успешного handoff acceptance.

Если `response_output_handoff_to_writer_wait_ms` присутствует, это поле MUST описывать только server-side интервал между `response_output_handoff_enqueued_at_ms` и `response_output_enqueue_completed_at_ms` и MUST NOT трактоваться как writer-queue backlog или конкретный blocker class без дополнительных authoritative fields.

Compatibility field `response_ready_to_output_enqueue_wait_ms` MAY сохраняться как umbrella интервал между `response_sent_at_ms` и `response_output_enqueue_completed_at_ms`, но MUST NOT переопределяться как точный синоним одного из новых `v24` buckets.

`response_output_enqueue_completed_at_ms` MUST NOT переосмысляться как truthful send-side enqueue completion для `v24`; это legacy compatibility field с writer-selection semantics, несмотря на историческое имя.

#### Scenario: VS Code клиент получает `v24` payload без reconstruction

- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v24` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Post-handler handoff gap отделён на три truthful bucket

- **GIVEN** completion handler уже подготовил response, outbound handoff начнётся позже, send-side acceptance завершится ещё позже, а output writer выберет completion response позже этого
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload сохраняет `response_sent_at_ms` и legacy `response_output_enqueue_completed_at_ms`
- **AND** публикует `response_output_handoff_started_at_ms`, `response_output_handoff_enqueued_at_ms`, `response_ready_to_output_handoff_wait_ms`, `response_output_handoff_send_wait_ms` и `response_output_handoff_to_writer_wait_ms` отдельно, если handoff boundaries наблюдаемы

#### Scenario: Legacy `response_output_enqueue_completed_at_ms` не выдаётся за truthful enqueue acceptance

- **GIVEN** authoritative payload содержит новый `v24` handoff split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_output_enqueue_completed_at_ms` трактуется как legacy writer-selection seam
- **AND** truthful send-side acceptance публикуется только через `response_output_handoff_enqueued_at_ms`

#### Scenario: Compatibility enqueue wait остаётся umbrella, а не переименованным bucket

- **GIVEN** authoritative payload содержит новый `v24` handoff split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `response_ready_to_output_enqueue_wait_ms` сохраняет compatibility semantics для полного интервала `response_sent_at_ms -> response_output_enqueue_completed_at_ms`
- **AND** consumer не трактует `v23` payload как будто truthful handoff boundaries уже были доступны

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v24`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v21` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v24/v21`, а не более старую версию

### Requirement: Timeline stage taxonomy bounded и совместима с completion observability (MUST)
Stage names в per-request timeline MUST использовать bounded taxonomy, согласованную с completion stage observability.

Timeline MUST NOT включать high-cardinality stage labels (динамические URI/пути/произвольные тексты) как часть имени stage.

#### Scenario: Stage labels остаются low-cardinality при разных файлах
- **GIVEN** completion выполняется для разных документов и рабочих областей
- **WHEN** формируются timeline traces
- **THEN** stage names остаются в пределах фиксированного словаря
- **AND** payload не содержит stage-name взрыва по cardinality

### Requirement: Timeline retention bounded и deterministic (MUST)
Per-request completion timeline хранилище MUST быть bounded по количеству записей (count-based ring buffer), с deterministic eviction oldest-first.

Retention default MUST быть задан как `max_entries=200`.

#### Scenario: Переполнение retention удаляет самые старые traces
- **GIVEN** в timeline buffer уже хранится `max_entries` traces
- **WHEN** добавляется новый completion trace
- **THEN** удаляется самый старый trace
- **AND** новые traces остаются доступными через `bsl.getCompletionTimeline`

### Requirement: Timeline instrumentation не меняет completion semantics и SLO-инварианты (MUST)
Запись per-request timeline MUST быть side-effect-safe:
- не должна менять user-facing completion response semantics;
- не должна добавлять блокирующий sync compute в request path;
- при внутренней ошибке timeline capture completion MUST продолжать работу в fail-open режиме для пользователя.

#### Scenario: Ошибка timeline capture не ломает completion ответ
- **GIVEN** во время записи timeline произошла внутренняя ошибка instrumentation
- **WHEN** completion pipeline формирует ответ пользователю
- **THEN** completion response возвращается по обычному контракту
- **AND** ошибка instrumentation не приводит к падению LSP completion handler

### Requirement: Canonical IR и derived semantic index образуют единый semantic core v2 (MUST)
Система MUST иметь единый semantic core вида `canonical IR -> derived semantic index`.

Canonical IR MUST содержать или однозначно порождать semantic facts, достаточные для:
- owner/member resolution;
- type-at-position;
- completion candidate semantics;
- definition/reference anchors, где требуется semantic ownership;
- diagnostics;
- flow-sensitive overlays через CFG.

`derived semantic index` MUST:
- строиться только из canonical IR snapshot текущей revision;
- быть детерминированной projection того же snapshot;
- не выполнять самостоятельный semantic inference;
- не читать `parse_result.program` как самостоятельный semantic source of truth.

#### Scenario: Один canonical IR snapshot даёт один semantic index для всех consumers
- **GIVEN** построен canonical IR snapshot конкретной revision
- **WHEN** система материализует derived semantic index для interactive queries
- **THEN** индекс является projection того же snapshot
- **AND** все consumers читают semantic facts из одного и того же IR-derived contract

#### Scenario: Derived semantic index не переизобретает semantic truth
- **GIVEN** canonical IR snapshot уже содержит owner/member/type truth для позиции
- **WHEN** derived semantic index строится для этой revision
- **THEN** индекс лишь денормализует lookup для fast queries
- **AND** не вычисляет альтернативный semantic результат из parse tree или отдельной эвристики

### Requirement: Facet-aware semantic identity сохраняется в canonical pipeline (MUST)
Для configuration types canonical IR + derived semantic index MUST сохранять facet-aware semantic identity, необходимую для owner/member/property resolution, hover, diagnostics и module-context bindings.

Этот contract MUST сохранять `active_facet` / `available_facets` или семантически эквивалентное представление.
`derived semantic index` MAY денормализовать facet lookup для fast queries, но MUST NOT сплющивать configuration type до plain metadata/platform type name, если это меняет semantic members, properties или owner behavior.

#### Scenario: ObjectModule explicit binding сохраняет object facet semantics
- **GIVEN** код в `ObjectModule` использует `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover` или `members`
- **THEN** semantic result сохраняет object-facet identity owner type текущего модуля
- **AND** member/property lookup использует object semantics, а не manager/reference substitute

#### Scenario: RecordSetModule explicit binding сохраняет recordset facet semantics
- **GIVEN** код в `RecordSetModule` использует `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover`, `members` или diagnostics для member access
- **THEN** semantic result сохраняет recordset object facet текущего owner type
- **AND** canonical pipeline не теряет members/properties, зависящие от facet-aware lookup

### Requirement: Semantic fast index отделён от discovery/search read-model (MUST)
Система MUST различать:
- semantic fast index для interactive semantic queries;
- discovery/search read-model (`IndexSnapshot` и эквиваленты) для search/discovery сценариев.

Discovery/search read-model MAY сосуществовать в том же runtime, но MUST NOT быть semantic source of truth для `completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`, `diagnostics`.
Недоступность semantic fast index MUST NOT приводить к backfill через discovery/search read-model.

#### Scenario: Search index не подменяет semantic truth
- **GIVEN** в runtime одновременно существуют canonical IR-derived semantic index и discovery/search index
- **WHEN** IDE запрашивает `hover` или `completion`
- **THEN** semantic ответ строится только из canonical IR и semantic fast index текущей revision
- **AND** наличие search index не меняет semantic contract интерактивного ответа

#### Scenario: Search index не становится rescue path при miss semantic fast index
- **GIVEN** discovery/search index доступен, но semantic fast index текущей revision ещё недоступен
- **WHEN** IDE запрашивает `completion`, `hover` или `definition`
- **THEN** сервер отвечает fail-closed для текущей revision
- **AND** не строит semantic payload из discovery/search read-model

### Requirement: Adapter surfaces не реконструируют semantic truth локально (MUST)
LSP/Web/MCP/CLI surfaces MUST использовать shared semantic runtime contract как единственный semantic read path.

Adapters MAY:
- выполнять syntax/position extraction;
- конвертировать spans/offsets в surface-specific coordinates;
- формировать transport payload.

Adapters MUST NOT:
- реконструировать owner/member/type truth локально из `parse_result`;
- использовать текстовые эвристики как substitute для semantic truth;
- использовать adapter-local caches или precomputed artifacts как stale substitute после смены revision;
- materialize-ить alternate semantic answer при miss canonical artifacts.

#### Scenario: Adapter miss остаётся fail-closed
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** любой adapter surface запрашивает `completion`, `hover`, `definition` или `type-at-position`
- **THEN** surface возвращает fail-closed результат согласно своему API contract
- **AND** не строит локальный semantic substitute вне shared runtime path

### Requirement: Canonical semantic queries fail-closed при недоступности артефактов (MUST)
Interactive semantic queries (`completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`) MUST завершаться fail-closed, если для них недоступен необходимый canonical current-revision artifact.

Требуемые артефакты:
- `completion` -> `CompletionHeadArtifact` ИЛИ `ExactSemanticArtifact`;
- `hover`, `signatureHelp`, `definition`, `type-at-position` -> `ExactSemanticArtifact`.

Fail-closed path MUST NOT:
- использовать stale semantic artifacts как substitute;
- возвращать semantic payload предыдущей revision под видом ответа для текущей revision;
- возвращать keyword fallback как semantic answer;
- запускать альтернативный parse-result-based semantic inference path;
- усиливать semantic truth локальной adapter logic.

#### Scenario: Hover miss current revision остаётся fail-closed
- **GIVEN** exact semantic artifact текущей revision недоступен
- **WHEN** IDE запрашивает hover в позиции с member access
- **THEN** сервер возвращает empty/unavailable hover response
- **AND** не materialize-ит semantic ответ из `CompletionHeadArtifact` или другого non-exact пути

#### Scenario: Completion head current revision допустим, stale exact другой revision недопустим
- **GIVEN** `CompletionHeadArtifact` для текущей revision ready
- **AND** exact semantic artifact ready только для предыдущей revision
- **WHEN** IDE запрашивает completion
- **THEN** сервер использует только current-revision `CompletionHeadArtifact` или fail-closed
- **AND** не использует exact artifact предыдущей revision как substitute

### Requirement: Fail-closed observability использует bounded reason codes (MUST)
Когда interactive semantic запрос завершается fail-closed, observability MUST фиксировать bounded low-cardinality reason code для текущей revision.

Reason code MUST описывать причину недоступности canonical path и MUST NOT обозначать alternate semantic path как допустимый substitute.
Reason taxonomy MUST оставаться low-cardinality и одинаково интерпретироваться во всех interactive surfaces.

#### Scenario: Miss current revision отражается bounded reason code
- **GIVEN** canonical IR или derived semantic index текущей revision недоступны
- **WHEN** IDE запрашивает `hover` или `completion`
- **THEN** observability фиксирует bounded reason code для fail-closed результата
- **AND** причина не маскирует ответ как stale-but-acceptable semantic path

### Requirement: Interactive latency budget защищается canonical fast path, а не fallback semantics (MUST)
Система MUST удовлетворять согласованным representative latency budgets для interactive semantic queries с использованием canonical IR и canonical derived semantic artifacts.

Для completion latency budget MAY соблюдаться через current-revision `CompletionHeadArtifact`, но MUST NOT соблюдаться через stale, degraded или discovery-backed semantic substitute.

Если latency budget нарушен, система MUST оптимизировать canonical semantic path и MUST NOT возвращать stale, degraded или discovery-backed semantic substitute как механизм соблюдения latency.

#### Scenario: Representative large-module completion использует canonical head path, а не stale rescue
- **GIVEN** representative large real module
- **WHEN** команда исправляет latency interactive completion
- **THEN** first-response completion приходит из current-revision `CompletionHeadArtifact` или `ExactSemanticArtifact`
- **AND** merge-state не вводит stale/degraded/search-backed semantic substitute как perf workaround

### Requirement: Applied-owner bare identifier fallback удалён из v2 semantics (MUST)
Система MUST NOT резолвить bare identifiers в `ObjectModule` / `RecordSetModule` через special applied-owner fallback вне canonical IR semantic binding model.

Если implicit module-context identifier semantics нужны продукту, они MUST быть представлены в canonical IR/binding model и одинаково доступны всем consumers.

#### Scenario: Explicit module-context bindings остаются canonical после удаления fallback
- **GIVEN** код в `ObjectModule` или `RecordSetModule` использует explicit context identifier `ЭтотОбъект` или `Объект`
- **WHEN** система выполняет `type-at-position`, `hover`, `definition`, `members` или diagnostics для member access от этого identifier
- **THEN** `ЭтотОбъект` / `Объект` резолвятся через canonical IR/binding model текущей revision
- **AND** owner/member semantics для такого доступа одинаковы во всех consumers
- **AND** система не зависит от applied-owner bare identifier fallback branch

#### Scenario: Bare identifier без canonical binding остаётся unresolved
- **GIVEN** код в `ObjectModule` или `RecordSetModule` содержит bare identifier, который не имеет canonical binding в текущем snapshot
- **WHEN** система выполняет type-at-position, hover или diagnostics для этого identifier
- **THEN** identifier остаётся unresolved согласно canonical semantic contract
- **AND** система не резолвит его через applied-owner fallback branch

### Requirement: Shared resolved contract first-class выражает snapshot-local structural members (MUST)
Система MUST представлять snapshot-local structural knowledge для typed `Структура` и typed-row в shared resolved contract, доступном всем semantic consumers.

Structural member entry MUST содержать как минимум:
- canonical member name;
- stable member identity;
- member type;
- certainty;
- source span или эквивалентную source location.

Representation только через generic `base_type` и неименованные type parameters MUST NOT считаться достаточной shared truth для structural members.

#### Scenario: Typed structure member существует как first-class shared data
- **GIVEN** snapshot содержит typed `Структура` с полем, появившимся из snapshot-local effect
- **WHEN** любой consumer запрашивает owner/member semantics для этого поля
- **THEN** shared resolved contract содержит first-class structural member entry
- **AND** consumer не вынужден восстанавливать имя/тип поля из локальной эвристики

#### Scenario: Typed-row column существует как first-class shared data
- **GIVEN** snapshot содержит typed-row `ТаблицаЗначений`
- **WHEN** consumer резолвит колонку как свойство строки
- **THEN** shared resolved contract содержит first-class entry для этой колонки
- **AND** generic/base-type-only representation недостаточна как единственный источник истины

### Requirement: Semantic consumers используют один resolved path или thin adapters (MUST)
`completion`, `hover`, `type-at-position`, `semantic diagnostics`, а также adapter surfaces (`LSP`, `MCP`, Web) MUST использовать один semantic resolved path в рамках одного snapshot/revision.

Consumer-local ветки допустимы только как thin adapters:
- преобразуют output shape;
- не вводят собственную schema/effect truth;
- не требуют локального semantic восстановления owner/member знания как условия корректности.

Если временное исключение сохраняется, оно MUST быть явно перечислено в approved migration plan и MUST иметь стратегию удаления.

#### Scenario: Completion не требует hidden local owner-resolution branch для shared semantics
- **GIVEN** owner/member semantics уже присутствуют в shared resolved contract
- **WHEN** completion формирует candidates
- **THEN** completion читает тот же resolved path, что и hover/type-at-position/diagnostics
- **AND** результат не зависит от отдельной consumer-local schema/effect ветки

### Requirement: Cross-consumer acceptance доказывает semantic equivalence, а не только smoke consistency (MUST)
Acceptance для shared semantic contract MUST включать exact assertions, которые подтверждают одну и ту же semantic truth между consumers.

Минимально acceptance MUST уметь доказать:
- одинаковый owner resolution результат;
- одинаковую member identity;
- одинаковую known/unknown policy для одного и того же доступа;
- отсутствие обязательных hidden hints, принадлежащих только одному consumer.

Smoke/parity проверки MAY использоваться как дополнительный слой, но MUST NOT быть единственным доказательством общей semantic truth.

#### Scenario: Acceptance выявляет hidden consumer-only hint path
- **GIVEN** один consumer получает корректный member только при локальном hint, недоступном другим consumers
- **WHEN** выполняется exact cross-consumer acceptance
- **THEN** acceptance падает как semantic drift
- **AND** smoke-level parity без этой проверки не считается достаточным evidence

### Requirement: Completion transport/cancellation observability остаётся bounded и completion-specific (MUST)
Server-side observability для transport/cancellation diagnostics MUST оставаться bounded и completion-specific.

Instrumentation MUST:
- записывать bounded latency samples для `transport_to_handler_wait`;
- записывать bounded latency samples для `server_handler_exec`;
- записывать bounded cancellation observability только для completion path;
- не вводить high-cardinality metric labels или free-form cancellation reasons.

#### Scenario: Cancellation observability не взрывает cardinality
- **GIVEN** completion requests отменяются для разных документов и запросов
- **WHEN** сервер пишет transport/cancellation observability
- **THEN** новые metric keys остаются в fixed low-cardinality vocabulary
- **AND** URI, snippets и произвольные reason strings не попадают в metric labels

### Requirement: `v9` pre-service-scope split сохраняет trustworthy attribution semantics из `v8` (MUST)
Новый bounded split MUST не ослаблять existing `v8` integrity semantics для pre-method attribution.

Сервер MUST:
- сохранять existing `pre_method_attribution_provenance`;
- не подменять отсутствие `v9` split guessed полями;
- не добавлять free-text/high-cardinality debug fields.

#### Scenario: Connected server ещё не поддерживает `v9`
- **GIVEN** connected server возвращает completion timeline `v8`
- **WHEN** extension или operator читает authoritative payload
- **THEN** payload не выдумывает `service_future_created_at_ms`
- **AND** trustworthy provenance semantics остаются ограничены уже существующими `v8` полями

### Requirement: `v10` dispatch split сохраняет truthful ingress provenance и honest fallback semantics (MUST)
Новый bounded dispatch split MUST не ослаблять existing `v9` integrity semantics для ingress attribution.

Сервер MUST:
- сохранять existing `pre_method_attribution_provenance`;
- не подменять отсутствие outer dispatch timestamp guessed полями;
- не добавлять free-text/high-cardinality debug fields;
- явно сообщать provenance ingress anchor через `transport_received_at_ms_provenance`.

#### Scenario: Outer dispatch hook недоступен для конкретного trace
- **GIVEN** completion timeline trace не содержит authoritative outer dispatch timestamp
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload честно помечает `transport_received_at_ms_provenance=request_context_call_entry`
- **AND** не выдумывает `jsonrpc_dispatch_received_at_ms`
- **AND** не выдумывает `dispatch_to_request_context_wait_ms`

#### Scenario: Connected server ещё не поддерживает `v10`
- **GIVEN** connected server возвращает completion timeline `v9`
- **WHEN** extension или operator читает authoritative payload
- **THEN** payload не выдумывает dispatch-to-request-context split
- **AND** trustworthy semantics остаются ограничены уже существующими `v9` полями

### Requirement: Completion timeline `v8` публикует trustworthy pre-method attribution provenance (MUST)
Authoritative `bsl.getCompletionTimeline` payload MUST поднимать contract до `v8`, если он переносит pre-method attribution provenance.

Если payload включает bounded pre-method facts (`service_scope_entered_at_ms`, `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`), он MUST также включать bounded provenance для этих фактов. Provenance vocabulary MUST оставаться low-cardinality и MUST различать как минимум:
- same-request authoritative attribution;
- best-effort fallback attribution.

Payload MUST NOT выдавать best-effort fallback за доказанный same-request pre-method факт.

#### Scenario: Overlapping completion на одной позиции не получает чужой authoritative ingress
- **GIVEN** два completion request пересекаются на одном и том же `uri + position`
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** trace не маркирует pre-method attribution как same-request authoritative, если provenance не доказан для этого `request_id`
- **AND** payload не маскирует best-effort fallback под strong ingress факт

#### Scenario: Request-bound attribution сохранён через service handoff
- **GIVEN** completion request сохраняет свой request context до consumer path
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** payload включает bounded provenance same-request authoritative attribution
- **AND** оператор может доверять pre-method split как факту для этого `request_id`

#### Scenario: Provenance недоступен
- **GIVEN** completion trace не может доказать provenance pre-method attribution
- **WHEN** сервер сериализует completion timeline `v8`
- **THEN** payload явно деградирует до bounded fallback/unavailable semantics
- **AND** не выдумывает strong same-request attribution

### Requirement: Pre-method attribution integrity остаётся bounded и side-effect-safe (MUST)
Integrity instrumentation для pre-method attribution MUST:
- не менять completion semantics;
- не добавлять новый unbounded лог-канал;
- использовать только bounded fields и bounded vocabulary;
- fail-open для самого completion response и fail-closed для attribution confidence.

#### Scenario: Timeline не может сохранить request-bound provenance
- **GIVEN** completion response всё ещё может быть построен, но request-bound provenance для pre-method attribution потерян
- **WHEN** timeline trace формируется
- **THEN** completion response пользователю остаётся прежним
- **AND** timeline понижает confidence или опускает strong attribution
- **AND** payload не заменяет missing provenance guessed полями

### Requirement: Completion timeline v7 сужает `server_before_method_entry` до bounded pre-method segments (MUST)
Authoritative `bsl.getCompletionTimeline` payload MUST поднимать contract до `v7` и MUST сохранять существующие server-edge fields, дополняя их bounded pre-method split без free-text логов.

Если payload включает новый pre-method split, он MUST использовать только additive bounded поля:
- optional `service_scope_entered_at_ms`;
- optional `transport_to_service_scope_wait_ms`;
- optional `service_scope_to_method_wait_ms`.

Если `service_scope_entered_at_ms` присутствует, payload MUST включать и оба derived waits, чтобы оператору не приходилось вручную вычитать timestamp'ы.

#### Scenario: Запрос задерживается до первого poll service future
- **GIVEN** completion request получает большой lag до начала service future
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload содержит bounded pre-method split
- **AND** `transport_to_service_scope_wait_ms` показывает положительную задержку
- **AND** старые поля `transport_to_method_wait_ms` и `transport_to_handler_wait_ms` остаются доступны

#### Scenario: Запрос задерживается между первым poll и входом в `lsp_completion`
- **GIVEN** completion request уже вошёл в service future scope, но ещё не достиг первой строки `lsp_completion`
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload содержит положительный `service_scope_to_method_wait_ms`
- **AND** оператор может отличить этот случай от lag до первого poll

### Requirement: `prepare_timeout` на `snapshot_with_deps` получает timeout-safe bounded runtime attribution (MUST)
Если `prepare_timeout` происходит после входа в фазу `snapshot_with_deps`, authoritative payload MUST уметь сериализовать bounded `snapshot_with_deps_timeout_runtime`, достаточный для различения overshoot как минимум между:
- `queue_wait`
- `exec`
- `wake_wait`
- `unavailable`

Object MUST оставаться bounded и MAY включать только:
- optional `queue_wait_ms`
- optional `exec_ms`
- optional `wake_wait_ms`
- required `resolution`

#### Scenario: Timeout происходит во время queue wait snapshot command
- **GIVEN** completion prepare timeout случается, пока `GetSnapshotWithDeps` ещё ждёт исполнения в runtime queue
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload включает `snapshot_with_deps_timeout_runtime`
- **AND** `resolution=queue_wait`
- **AND** payload не выдумывает `exec_ms`, если exec ещё не начался

#### Scenario: Timeout происходит после готового snapshot reply, но до timely wake
- **GIVEN** runtime уже завершил snapshot command, но completion future просыпается слишком поздно
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload включает bounded `snapshot_with_deps_timeout_runtime`
- **AND** `resolution=wake_wait`

#### Scenario: Timeout path ещё не имеет partial runtime split
- **GIVEN** prepare timeout произошёл на `snapshot_with_deps`, но bounded partial runtime attribution пока недоступна
- **WHEN** сервер сериализует completion timeline `v7`
- **THEN** payload использует `resolution=unavailable`
- **AND** не подменяет отсутствие данных guessed queue/exec/wake split

### Requirement: Human-readable completion ingress verdicts остаются truthful и positive-only (MUST)
Derived verdicts для `Completion Timeline` panel, clipboard и связанных extension projections MUST строиться только из уже имеющихся bounded latency fields и MUST NOT маркировать trace как ingress-bottleneck, если соответствующая ingress задержка отсутствует.

Derived verdict layer MUST:
- использовать только существующие bounded waits (`adapter_to_dispatch_wait_ms`, `transport_to_method_wait_ms`, `method_prelude_exec_ms` и, при наличии deterministic correlation в downstream consumer, `client_to_transport_wait_ms`);
- строить ingress verdict только при положительной доминирующей задержке;
- различать как минимум `adapter_before_dispatch_dominant`, `server_before_method_entry_dominant` и `handler_prelude_dominant`;
- MAY различать `client_before_transport_dominant`, если downstream projection уже имеет deterministic probe correlation и authoritative earliest server ingress boundary;
- не выводить generic ingress verdict только потому, что `0 >= 0` или потому что одна из задержек отсутствует.

#### Scenario: Adapter wait доминирует над dispatch-to-method и handler prelude
- **GIVEN** completion trace имеет положительный `adapter_to_dispatch_wait_ms`, который доминирует над `transport_to_method_wait_ms` и `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `adapter_before_dispatch_dominant`
- **AND** trace не получает `client_before_transport_dominant` только из-за позднего dispatch timestamp

#### Scenario: Hot trace без положительного ingress wait не получает ingress verdict
- **GIVEN** completion trace имеет `adapter_to_dispatch_wait_ms=0`, `transport_to_method_wait_ms=0` и `method_prelude_exec_ms=0`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает ingress verdict
- **AND** trace не маркируется как `handler_prelude_dominant`

#### Scenario: Handler prelude доминирует над server-side waits
- **GIVEN** completion trace имеет положительный `method_prelude_exec_ms`, который доминирует над `adapter_to_dispatch_wait_ms` и `transport_to_method_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `handler_prelude_dominant`
- **AND** trace не получает `adapter_before_dispatch_dominant`

### Requirement: Client-side ingress supplement остаётся fail-closed и deterministic (MUST)
Если extension-projection добавляет human-readable client-side ingress verdict поверх authoritative completion trace, такой verdict MUST появляться только при deterministic probe correlation и положительном доминирующем client-side wait до самой ранней authoritative server ingress boundary.

Проекция MUST:
- не создавать client-side ingress verdict для uncorrelated или ambiguous requests;
- использовать `adapter_read_at_ms` как server ingress boundary, если payload её содержит;
- использовать более поздний `transport_received_at_ms` только как backward-compatible fallback для старых payload'ов, где ранняя adapter boundary отсутствует;
- не использовать probe-only эвристики как substitute для authoritative server verdicts;
- сохранять trace валидным и server-centric, если client correlation недоступна.

#### Scenario: Pre-dispatch server backlog не публикуется как client-side ingress
- **GIVEN** request summary имеет deterministic correlation
- **AND** authoritative payload содержит положительный `adapter_to_dispatch_wait_ms`
- **AND** положительный wait до ранней adapter boundary не доказан
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает verdict `client_before_transport_dominant`
- **AND** projection остаётся fail-closed по client-side supplement

#### Scenario: Legacy payload без adapter boundary сохраняет bounded fallback
- **GIVEN** request summary имеет deterministic correlation, но connected server возвращает более старый payload без `adapter_read_at_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** projection MAY использовать bounded legacy fallback на `transport_received_at_ms`
- **AND** verdict не публикуется, если deterministic client-side delay всё равно не доказан

### Requirement: Existing completion surfaces переносят `v6` root-cause attribution без invented data (MUST)
VS Code extension MUST переносить authoritative `v6` root-cause attribution в уже существующие completion-oriented surface'ы, не требуя от оператора ручного чтения raw JSON для типовых verdict'ов.

Минимальные surface'ы:
- Completion Timeline panel;
- clipboard export видимого trace;
- observability incident handoff summary поверх authoritative timeline.

Derived projection MUST:
- строиться только из structured authoritative fields и bounded local status markers;
- различать `ingress_before_method_entry`, `handler_prelude_dominant`, `prepare_timeout@source` и `exact_deadline@artifact_poll`, когда соответствующие поля доступны;
- явно деградировать на payload `v5`;
- MUST NOT придумывать отсутствующие значения и MUST NOT подменять raw attachments.

#### Scenario: Completion Timeline panel показывает method-entry и timeout attribution
- **GIVEN** сервер вернул completion timeline с `v6` root-cause attribution
- **WHEN** пользователь открывает Completion Timeline panel
- **THEN** panel показывает bounded fact lines для method-entry split, timeout source/overshoot и artifact polling, если эти поля присутствуют
- **AND** оператору не требуется открывать raw JSON для типовых verdict'ов `handler_prelude_dominant`, `prepare_timeout@source` или `exact_deadline@artifact_poll`

#### Scenario: Clipboard export переносит ключевой `v6` verdict
- **GIVEN** пользователь копирует trace из Completion Timeline
- **WHEN** extension формирует clipboard text
- **THEN** copied text содержит ключевые bounded `v6` fact lines
- **AND** copied text не теряет distinction между transport wait, method prelude, timeout source и artifact polling, если эти поля присутствуют

#### Scenario: Incident handoff summary деградирует явно на payload `v5`
- **GIVEN** extension строит incident handoff summary для backend, который ещё не вернул `v6` root-cause attribution
- **WHEN** summary формируется из completion timeline payload версии `5`
- **THEN** summary остаётся валидным и использует доступные `v5` поля
- **AND** отсутствующие `v6` verdict details помечаются как unavailable, а не выдумываются

### Requirement: Человекочитаемые completion timeline projections сохраняют authoritative bottleneck semantics (MUST)
VS Code extension MUST проецировать bounded authoritative bottleneck drilldown из completion timeline в человекочитаемые surface'ы, не заставляя оператора читать raw JSON для типовых root-cause verdict'ов.

Минимальные surface'ы:
- Completion Timeline panel;
- clipboard export видимого trace;
- AI-friendly incident handoff summary поверх authoritative timeline.

Derived projection MUST:
- строиться только из structured authoritative fields и bounded local status markers;
- явно различать `ingress_dominant`, `prepare_timeout` subphase и `exact_wait` bottleneck, когда соответствующие поля доступны;
- деградировать явно, если backend вернул старый payload или часть новых bounded полей отсутствует;
- MUST NOT придумывать отсутствующие значения и MUST NOT подменять raw attachments.

#### Scenario: Completion Timeline panel показывает bounded bottleneck drilldown
- **GIVEN** сервер вернул completion timeline с bounded prepare/exact/dispatcher drilldown
- **WHEN** пользователь открывает Completion Timeline panel
- **THEN** panel показывает эти факты в человекочитаемом виде рядом с trace
- **AND** оператору не требуется открывать raw JSON для типового verdict `ingress_dominant`, `prepare_timeout@phase` или `exact_deadline`

#### Scenario: Clipboard export переносит ключевой bottleneck verdict
- **GIVEN** пользователь копирует trace из Completion Timeline
- **WHEN** extension формирует clipboard text
- **THEN** copied text содержит ключевые bounded drilldown поля
- **AND** copied text не теряет distinction между `transport_to_handler_wait`, `dispatcher_resolution_latency_ms`, `prepare` subphase и `exact_wait` state, если эти поля присутствуют

#### Scenario: Incident handoff summary деградирует явно на payload `v4`
- **GIVEN** extension строит incident handoff summary для backend, который ещё не вернул `v5` drilldown
- **WHEN** summary формируется из completion timeline payload более старой версии
- **THEN** summary остаётся валидным и использует доступные `v4` поля
- **AND** отсутствующие `v5` verdict details помечаются как unavailable, а не выдумываются

### Requirement: v2 отслеживает schema-effects universal value collections в одном snapshot (MUST)
Система MUST отслеживать snapshot-local schema/effects для universal value collections (`Соответствие`, `Структура`, `ТаблицаЗначений`) в рамках одного и того же v2 snapshot.

Эти effects MUST использоваться как единый source of truth для:
- `completion`,
- `hover`,
- `type-at-position`,
- `semantic diagnostics`.

#### Scenario: Один snapshot даёт единый смысл для collection-derived access
- **GIVEN** документ создаёт `Соответствие`, `Структура` и `ТаблицаЗначений` с локально известными schema/effects
- **WHEN** IDE выполняет completion, hover и diagnostics в одной ревизии документа
- **THEN** все операции используют один и тот же resolved type contract без расхождений между consumer-слоями

### Requirement: Consumer channels используют только единый resolved path (MUST)
Система MUST использовать один и тот же resolved owner/type contract для `completion`, `hover`, `type-at-position` и `semantic diagnostics`.

Consumer-local schema/effect inference MUST NOT использоваться как источник истины.
Допустим только thin-adapter, который читает уже резолвленный тип из общего v2 snapshot contract.

#### Scenario: Один и тот же owner-type для member access во всех consumer каналах
- **GIVEN** документ содержит member/index access, зависящий от schema-effects universal value collections
- **WHEN** IDE последовательно запрашивает completion, hover, type-at-position и semantic diagnostics в одной ревизии документа
- **THEN** все каналы используют один и тот же resolved owner/type contract
- **AND** результаты не расходятся из-за consumer-local inference

### Requirement: v2 резолвит значение `Соответствие` при index access (MUST)
Система MUST резолвить тип выражения `map[key]` для `Соответствие` в рамках одного v2 snapshot, используя map-effects из кода.

#### Scenario: Литеральный ключ резолвит точный тип значения
- **GIVEN** код содержит `Map = Новый Соответствие;` и `Map.Вставить("Идентификатор", 10)`
- **WHEN** анализируется выражение `Map["Идентификатор"]`
- **THEN** система возвращает тип `Число`

### Requirement: Приоритет резолюции `map[key]` определён и стабилен (MUST)
Система MUST применять следующий порядок для определения типа `map[key]`:
1. тип literal-key specialization для конкретного ключа,
2. generic value type `V`,
3. fallback `Произвольный`.

#### Scenario: Неизвестный ключ использует generic `V`
- **GIVEN** map типизирован как `Соответствие<Строка, ДокументСсылка.Док1>`
- **AND** literal-key specialization для ключа отсутствует
- **WHEN** анализируется `Map["ЛюбойКлюч"]`
- **THEN** система возвращает тип `ДокументСсылка.Док1`

#### Scenario: При отсутствии данных используется `Произвольный`
- **GIVEN** map создан как `Новый Соответствие` без достаточных эффектов вывода
- **WHEN** анализируется `Map[Expr]`
- **THEN** система возвращает тип `Произвольный`

### Requirement: Completion/hover/type-at-position после map index access согласованы (MUST)
Система MUST использовать один и тот же resolved owner-type после index access для completion, hover и type-at-position.

#### Scenario: Completion на `map["k"].` использует тип значения
- **GIVEN** `Map.Вставить("k", Obj)` и тип `Obj` содержит свойство `Имя`
- **WHEN** IDE запрашивает completion на `Map["k"].`
- **THEN** completion включает `Имя`

#### Scenario: Hover/type-at-position на `map["k"]` показывает тип значения
- **GIVEN** для `k` выведен тип `КолонкаТаблицыЗначений`
- **WHEN** IDE запрашивает hover/type-at-position на `Map["k"]`
- **THEN** система возвращает тип `КолонкаТаблицыЗначений`

### Requirement: Dynamic keys не генерируют hard-fail о неизвестном ключе (MUST)
Для динамических ключей система MUST избегать hard-fail диагностики "ключ не найден" и использовать safe type fallback policy.

#### Scenario: Динамический ключ не вызывает ложную ошибку отсутствия ключа
- **GIVEN** ключ вычисляется как выражение `Ключ = ПолучитьКлюч()`
- **WHEN** анализируется `Map[Ключ]`
- **THEN** система не создаёт hard-fail диагностику о неизвестном ключе
- **AND** тип выражения определяется по policy приоритетов

### Requirement: Per-instance schema не мутирует глобальный TypeRepository (MUST)
Per-instance schema/effects MUST оставаться snapshot-local.
Система MUST NOT регистрировать synthetic per-instance типы или иные mutable записи в глобальном `TypeRepository` ради `Соответствие` / `Структура` / `ТаблицаЗначений`.

#### Scenario: Переключение snapshot не протекает per-instance state в global repository
- **GIVEN** один snapshot содержит локально выведенную schema для экземпляров universal value collections
- **AND** другой snapshot не содержит эту schema
- **WHEN** выполняется анализ во втором snapshot
- **THEN** во втором snapshot не появляются synthetic members из первого snapshot
- **AND** global `TypeRepository` не содержит новых per-instance synthetic типов

### Requirement: v2 отслеживает flow-sensitive schema полей `Структура` (MUST)
Система MUST отслеживать schema-effect полей конкретного экземпляра `Структура` в рамках одного v2 snapshot.

Отслеживание MUST как минимум включать:
- имя поля (регистронезависимый lookup с сохранением каноничного имени),
- тип поля (если извлечён),
- source span добавления/обновления поля.

#### Scenario: Поле, добавленное через `Вставить`, доступно как свойство
- **GIVEN** код содержит `S = Новый Структура;` и `S.Вставить("Идентификатор", "A-01")`
- **WHEN** v2 строит type snapshot для completion/hover/diagnostics
- **THEN** схема `S` содержит поле `Идентификатор`
- **AND** member access `S.Идентификатор` резолвится без ошибки

### Requirement: Typed-structure резолвит поля как свойства в user-facing каналах (MUST)
Для переменной, определённой как typed-structure, система MUST единообразно резолвить `s.<ИмяПоля>` в completion, hover и type-at-position.

#### Scenario: Completion предлагает известные поля структуры
- **GIVEN** у структуры `S` есть поля `Идентификатор` и `Количество`
- **WHEN** IDE запрашивает completion на `S.`
- **THEN** completion включает `Идентификатор` и `Количество`

#### Scenario: Hover/type-at-position возвращает тип поля структуры
- **GIVEN** поле `Идентификатор` имеет тип `Строка`
- **WHEN** IDE запрашивает hover/type-at-position для `S.Идентификатор`
- **THEN** система возвращает тип `Строка`

### Requirement: Unknown field у typed-structure диагностируется как ошибка (MUST)
Если объект определён как typed-structure, обращение к полю, отсутствующему в schema, MUST приводить к диагностике несуществующего свойства.

#### Scenario: Опечатка в имени поля даёт hard-fail диагностику
- **GIVEN** в schema структуры есть только поле `Идентификатор`
- **WHEN** код обращается к `S.Идентифкатор`
- **THEN** система возвращает диагностику о несуществующем свойстве для typed-structure

### Requirement: Тип поля извлекается best-effort с безопасным fallback (MUST)
Система MUST извлекать тип поля из поддерживаемых паттернов присваивания/вставки.
Если тип вычислить невозможно, система MUST сохранять поле в schema и использовать тип `Произвольный`.

#### Scenario: Невычислимый тип не удаляет поле из schema
- **GIVEN** код содержит `S.Вставить("СложноеПоле", ПолучитьНечёткийТип())`
- **WHEN** система не может статически вычислить тип значения
- **THEN** поле `СложноеПоле` остаётся доступным в member access/completion
- **AND** его тип определяется как `Произвольный`

### Requirement: v2 отслеживает schema-effect `ТаблицаЗначений.Колонки.Добавить` (MUST)
Система MUST отслеживать side-effect вызовов `ТЗ.Колонки.Добавить(...)` как изменение схемы колонок конкретного экземпляра `ТаблицаЗначений` в рамках одного v2 snapshot.

Отслеживание MUST как минимум включать:
- имя колонки (регистронезависимый lookup, с сохранением каноничного имени для отображения),
- тип значения колонки (если извлечен из аргументов),
- source span изменения схемы.

#### Scenario: Колонка, добавленная через `Колонки.Добавить`, видна в том же snapshot
- **GIVEN** код содержит `ТЗ = Новый ТаблицаЗначений;` и `ТЗ.Колонки.Добавить("Идентификатор", ...)`
- **WHEN** v2 строит type snapshot для diagnostics/completion/hover
- **THEN** схема `ТЗ` содержит колонку `Идентификатор`

### Requirement: Typed-row `ТаблицаЗначений` резолвит колонки как свойства (MUST)
Система MUST формировать typed-row для строк `ТаблицаЗначений` (минимум для `ТЗ.Добавить()` и `Для каждого Стр Из ТЗ`) и MUST резолвить `Стр.<ИмяКолонки>` по сохраненной схеме колонок.

#### Scenario: Completion предлагает колонки строки таблицы
- **GIVEN** таблица содержит колонки `Идентификатор`, `ИдентификаторОрганизации`, `ТипПоиска`
- **AND** переменная `Стр` является строкой этой таблицы
- **WHEN** IDE запрашивает completion на `Стр.`
- **THEN** completion включает `Идентификатор`, `ИдентификаторОрганизации`, `ТипПоиска`

#### Scenario: Hover/type-at-position возвращает тип колонки строки
- **GIVEN** тип колонки `Идентификатор` извлечен как `Строка`
- **WHEN** IDE запрашивает hover/type-at-position для `Стр.Идентификатор`
- **THEN** система возвращает тип `Строка`

### Requirement: Unknown column у typed-row диагностируется в strict режиме (MUST)
Если объект определен как typed-row `ТаблицаЗначений`, обращение к колонке, отсутствующей в схеме, MUST приводить к диагностике несуществующего свойства (hard-fail), а не silently-degrade в `Unknown`.

#### Scenario: Опечатка в имени колонки даёт ошибку
- **GIVEN** в схеме строки есть только колонка `Идентификатор`
- **WHEN** код обращается к `Стр.Идентифкатор`
- **THEN** система возвращает диагностику о несуществующем свойстве для typed-row

### Requirement: Тип колонки извлекается из `ОписаниеТипов` с безопасной деградацией (MUST)
Система MUST извлекать тип колонки из аргументов `Колонки.Добавить` в поддерживаемых паттернах `ОписаниеТипов`.
Если извлечь тип невозможно, система MUST сохранять имя колонки и использовать тип `Произвольный`.

#### Scenario: `ОписаниеТипов` с StringType даёт `Строка`
- **GIVEN** `ОписаниеТиповСтрока150 = Новый ОписаниеТипов(КвалификаторыСтрок.StringType, ...)`
- **AND** `ТЗ.Колонки.Добавить("Идентификатор", ОписаниеТиповСтрока150)`
- **WHEN** IDE запрашивает тип `Стр.Идентификатор`
- **THEN** система возвращает `Строка`

#### Scenario: Неподдержанный `ОписаниеТипов` сохраняет колонку с `Произвольный`
- **GIVEN** `ТЗ.Колонки.Добавить("СложнаяКолонка", ВычислитьОписаниеТипов())`
- **WHEN** система не может извлечь тип колонки статически
- **THEN** колонка `СложнаяКолонка` остается доступной для member access/completion
- **AND** её тип определяется как `Произвольный`

### Requirement: Acceptance фиксирует cross-consumer consistency для одной позиции (MUST)
Система MUST иметь интеграционные acceptance tests, которые сравнивают результат для одной и той же позиции между `completion`, `hover`, `type-at-position` и `semantic diagnostics`.

#### Scenario: `map["k"].` консистентен между completion/hover/type-at-position/diagnostics
- **GIVEN** код выводит тип `map["k"]` из schema-effects
- **WHEN** запускаются acceptance tests по одной позиции в документе
- **THEN** completion candidates, hover/type-at-position type и semantic diagnostics соответствуют одному resolved owner/type contract

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- включать `didChange-burst` профиль через реальный LSP transport path, а не только прямой вызов service layer;
- отдельно учитывать `adapter_to_dispatch_wait_ms`, `service_future_to_first_poll_wait_ms`, first-response availability и exact upgrade latency;
- использовать warmup phase, которая не входит в measured set;
- собирать не менее 10 measured completion samples в `didChange-burst` профиле;
- fail-ить, если `p95(adapter_to_dispatch_wait_ms)` у measured completion samples выше `intellisense_v2_interactive_wait_budget_ms`;
- fail-ить, если любой measured sample имеет `adapter_to_dispatch_wait_ms > 4 * intellisense_v2_interactive_wait_budget_ms`;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path;
- fail-ить, если успешный first response достигается только после seconds-scale pre-dispatch backlog, вызванного concurrent general LSP traffic.

#### Scenario: Real-module gate ловит возврат pre-dispatch completion starvation
- **GIVEN** gate отправляет `didChange` churn и concurrent general LSP traffic через live transport path
- **WHEN** measured completion samples снова получают seconds-scale wait до dispatch
- **THEN** gate завершается ошибкой, даже если completion позже становится `ok_non_empty`
- **AND** отчёт выделяет pre-dispatch backlog отдельно от post-dispatch first-poll и handler latency

### Requirement: `v11` service-future poll / wake split сохраняет truthful post-dispatch attribution semantics (MUST)
Новый bounded split внутри `service_future_created -> service_scope_entered` MUST не ослаблять existing `v10` / `v9` / `v8` integrity semantics.

Сервер MUST:
- сохранять existing `transport_received_at_ms_provenance` и `pre_method_attribution_provenance`;
- не подменять отсутствие first-poll или first-wake observation guessed полями;
- не добавлять free-text/high-cardinality debug fields;
- явно сообщать bounded outcome первого poll через `service_future_first_poll_outcome`, если first poll наблюдался.

#### Scenario: Первый poll вернул `Pending`, но первый wake не наблюдался
- **GIVEN** completion timeline trace знает момент первого poll и знает, что он вернул `Pending`
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload честно включает `service_future_first_poll_outcome=pending`
- **AND** payload не выдумывает `service_future_first_wake_scheduled_at_ms`, если first wake не наблюдался
- **AND** payload не выдумывает `first_poll_to_first_wake_wait_ms`

#### Scenario: Connected server ещё не поддерживает `v11`
- **GIVEN** connected server возвращает completion timeline `v10`
- **WHEN** extension или operator читает authoritative payload
- **THEN** payload не выдумывает first-poll / first-wake split
- **AND** trustworthy semantics остаются ограничены уже существующими `v10` полями

### Requirement: LSP document-sync service future освобождает transport slot до slow background стадий (MUST)
`textDocument/didOpen` и `textDocument/didChange` MUST завершать свой service future после того, как:
- входной payload принят;
- `latest_received` и shadow state обновлены для новой requested revision;
- current-revision `SetFile` handoff зарегистрирован в analysis runtime writer path для той же `file_version`;
- минимальный handoff slow background work зарегистрирован;
- transport slot больше не удерживается ради ожидания slow background стадий.

`applied_version` в этом требовании продолжает означать revision, уже применённую в analysis runtime через `SetFile` / `SetFileWithSnapshot`. Она MUST NOT переопределяться как readiness `CompletionHeadArtifact`, `ExactSemanticArtifact` или diagnostics publish.

Для этого change current-revision handoff означает enqueue/register соответствующего `SetFile` в runtime writer path для той же `file_version`. Handoff сам по себе MUST NOT трактоваться как уже наблюдаемое продвижение `applied_version`.

После document-sync handoff `received_version` MAY уже указывать на новую requested revision, пока `applied_version` ещё кратко отстаёт и догоняет её через runtime writer path. Это допустимо для данного change при двух условиях:
- interactive orchestration продолжает использовать `applied_version` как критерий фактической готовности snapshot;
- `didOpen/didChange` не маскируют этот lag выдачей artifact-ready semantics под видом `applied_version`.

После этого slow стадии (`parse snapshot build`, current-revision completion precompute, exact precompute, deferred diagnostics) MUST продолжаться вне transport service future.

Document-sync path MUST NOT удерживать LSP transport request-admission slot только ради ожидания завершения этих slow стадий.

#### Scenario: `didChange` освобождает transport slot до завершения parse snapshot
- **GIVEN** changed-text `didChange` для большого модуля запускает дорогой `parse snapshot build`
- **WHEN** LSP принимает notification
- **THEN** document-sync service future завершается после current-revision handoff
- **AND** slow parse snapshot работа продолжается в фоне
- **AND** transport slot не удерживается до терминального завершения parse snapshot

#### Scenario: `didOpen` не ждёт slow parse/head path перед возвратом transport control
- **GIVEN** LSP открывает большой модуль, для которого initial parse/head path дорогой
- **WHEN** сервер принимает `textDocument/didOpen`
- **THEN** document-sync service future завершается после current-revision handoff
- **AND** slow parse/head/exact работа продолжается в фоне
- **AND** initial open не удерживает transport slot до терминального завершения slow path

#### Scenario: Handoff не приравнивает `received_version` к `applied_version`
- **GIVEN** `didChange` уже завершил service future после current-revision handoff
- **AND** `received_version=V+1`, но runtime writer path ещё не применил `SetFile` и `applied_version` остаётся `V`
- **WHEN** interactive completion запрашивается для той же requested revision `V+1`
- **THEN** orchestration продолжает ждать `applied_version >= V+1` bounded path'ом
- **AND** readiness `CompletionHeadArtifact` / `ExactSemanticArtifact` не считается substitute для `applied_version`

#### Scenario: `didChange` может завершиться до observable advance `applied_version`
- **GIVEN** changed-text `didChange` уже обновил `latest_received` и shadow state до `V+1`
- **AND** current-revision `SetFile` только поставлен в runtime writer path
- **WHEN** document-sync service future завершается
- **THEN** `received_version` MAY уже быть равен `V+1`
- **AND** `applied_version` MAY ещё кратко оставаться на `V`, пока runtime snapshot догоняет handoff
- **AND** это не считается нарушением short-lived transport contract

### Requirement: Current-revision readiness fast lane продвигает `applied_version` и `CompletionHeadArtifact` раньше slow enrich path (MUST)
После того как `textDocument/didOpen` или `textDocument/didChange` уже завершил свой transport service future и зарегистрировал current-revision handoff для `file_version=V`, система MUST считать interactive-critical минимумом для этого же `file_id`:
- продвижение `applied_version` до `V` через runtime writer path;
- публикацию и queryability `CompletionHeadArtifact` той же revision `V`.

Этот минимум MUST исполняться по readiness fast lane, который:
- получает приоритет над same-file и older-revision `type_index_precompute`, `ExactSemanticArtifact`, deferred diagnostics и прочими slow background стадиями, не являющимися prerequisite для first current-revision response;
- сохраняет latest-wins и supersession semantics для newest revision;
- MUST NOT публиковать stale semantic truth другой revision под видом current-revision readiness.

Post-handoff lag между registered handoff и observable advance `applied_version` MAY оставаться ненулевым, но completion MUST NOT тратить seconds-scale bounded wait только потому, что latest same-file apply стоит позади low-value background backlog.

`CompletionHeadArtifact` для current revision MUST NOT ждать готовности `ExactSemanticArtifact`, `type_index_precompute` или deferred diagnostics той же revision, если для first current-revision response они не обязательны. Exact upgrade MAY продолжаться в фоне.

#### Scenario: Newest same-file apply не ждёт старый background backlog
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для `file_version=V+1`
- **AND** в системе ещё выполняется older-revision `type_index_precompute` или diagnostics backlog
- **WHEN** completion запрашивается для `V+1`
- **THEN** runtime продвигает `applied_version` до `V+1` по readiness fast lane
- **AND** latest apply не остаётся ждать терминального завершения older background работы

#### Scenario: Current-revision head становится queryable до exact readiness
- **GIVEN** runtime уже продвинул `applied_version` до current revision `V`
- **AND** `ExactSemanticArtifact` для `V` ещё не ready
- **WHEN** completion запрашивается для той же revision `V`
- **THEN** `CompletionHeadArtifact` current revision остаётся publishable/queryable независимо от exact readiness
- **AND** exact upgrade продолжается в фоне

#### Scenario: Superseded readiness work не блокирует newest revision
- **GIVEN** same-file revision `V` уже имеет in-flight apply/head work
- **AND** приходит более новая revision `V+1`
- **WHEN** readiness scheduler перевыставляет latest work
- **THEN** superseded work для `V` не удерживает fast lane перед `V+1`
- **AND** user-facing readiness для `V+1` получает приоритет latest-wins

### Requirement: Completion first-response prepare разделяет lightweight current-revision path и exact stateful path (MUST)
Для member-access completion система MUST иметь отдельный current-revision prepare contract для first response, не эквивалентный generic heavy `prepare_stateful_operation`.

Этот контракт MUST уметь различать как минимум:
- `head-ready` для current-revision first response;
- `exact-ready` для full exact path;
- bounded `not-ready` для fail-closed path.

Lightweight current-revision prepare MUST:
- быть feature-specific и request-scoped;
- использовать только узкие immutable read-model/DTO данные, необходимые для first completion response;
- MUST NOT публиковать или кэшировать long-lived shared `AnalysisV2` как feature boundary.

#### Scenario: Current-revision head-ready path не требует heavy exact prepare
- **GIVEN** current revision уже имеет queryable `CompletionHeadArtifact`
- **AND** exact semantic path для той же revision еще не ready
- **WHEN** IDE запрашивает member-access completion
- **THEN** completion first response использует lightweight current-revision prepare
- **AND** не требует mandatory full exact stateful prepare как prereq для `head_hit`

#### Scenario: Lightweight prepare fail-closed при отсутствии current-revision truth
- **GIVEN** neither current-revision `CompletionHeadArtifact`, nor exact artifact не ready в пределах bounded policy
- **WHEN** IDE запрашивает member-access completion
- **THEN** completion завершает запрос bounded fail-closed
- **AND** не публикует stale или degraded semantic substitute

### Requirement: Superseded active completion освобождает interactive ownership до завершения stale response-build (MUST)
Если same-file completion request уже успел first-poll-нуться и войти в handler, но затем потерял latest-wins из-за более нового completion request или explicit cancel, система MUST перестать считать его владельцем active interactive completion path не позже ближайшего cooperative cancellation checkpoint после того, как supersession/cancel стал наблюдаемым.

Для этого completion pipeline MUST иметь interruption points, достаточные для prompt release stale active request внутри длинного `response_build` tail. Как минимум bounded interruptible contract MUST покрывать `collect`, `rank`, `format` и publish boundary либо эквивалентную implementation boundary с тем же observable результатом.

Этот contract MUST реализовываться на existing completion path. Новый admission workaround, отдельная transport/admission lane, увеличение concurrency само по себе или общий executor redesign MUST NOT считаться выполнением этого требования без prompt release stale active completion внутри существующего completion pipeline.

Superseded active request MUST NOT удерживать newer same-file completion в seconds-scale `service_future_created -> first poll` wait только потому, что stale `response_build` ещё не полностью завершился.

#### Scenario: Новый same-file completion first-poll-ится, пока старый request boundedly сворачивается
- **GIVEN** completion request `A` для файла уже вошёл в handler и начал тяжёлый `response_build`
- **AND** позже приходит более новый completion request `B` для того же файла
- **WHEN** request `A` теряет latest-wins
- **THEN** request `A` boundedly прекращает stale critical path на ближайшем cooperative checkpoint
- **AND** request `B` достигает first poll в пределах interactive policy, а не после seconds-scale stale tail request `A`

#### Scenario: Superseded response-build не публикует поздний user-facing completion
- **GIVEN** active completion request уже находится внутри `collect` / `rank` / `format`
- **WHEN** request получает explicit cancel или становится superseded более новым same-file request
- **THEN** stale request завершает ответ bounded cancelled/superseded outcome
- **AND** пользовательский completion ответ для этого stale request не публикуется поздно после потери актуальности

### Requirement: `v12` first-poll contention attribution остаётся bounded и fail-closed (MUST)
Новый bounded contender cut MUST давать только server-visible facts и MUST NOT подменять их guessed blocker claims.

Сервер MUST:
- использовать только low-cardinality contender vocabulary;
- не сериализовать request id, raw URI или free-text debug explanation внутри `first_poll_contention_attribution`;
- использовать `mixed`, если одновременно видимы несколько contender классов без честного single-class verdict;
- использовать `none_visible` или `unavailable`, если server-side snapshot не доказывает видимый contender class;
- не выдумывать `same_uri` / `other_uri`, если `uri_scope` нельзя доказать bounded way.

#### Scenario: Same-file document-sync видим до первого poll
- **GIVEN** completion trace долго ждёт первый poll
- **AND** server-side snapshot в этом окне видит contender class document-sync на том же `uri`
- **WHEN** сервер сериализует completion timeline `v12`
- **THEN** `first_poll_contention_attribution.contender_class=document_sync`
- **AND** `first_poll_contention_attribution.uri_scope=same_uri`
- **AND** payload остаётся bounded и не выдумывает точный blocking request id

#### Scenario: Одновременно видимы несколько contender классов
- **GIVEN** server-side snapshot в окне `service_future_created -> first_poll` видит больше одного contender class
- **WHEN** сервер сериализует completion timeline `v12`
- **THEN** payload использует `first_poll_contention_attribution.contender_class=mixed`
- **AND** payload не выбирает guessed "главного виновника"

#### Scenario: Contender snapshot не даёт доказанного класса
- **GIVEN** completion trace имеет положительный `service_future_to_first_poll_wait_ms`
- **AND** server-side snapshot не видит доказанного contender class или сам unavailable
- **WHEN** сервер сериализует completion timeline `v12`
- **THEN** payload использует bounded `none_visible` или `unavailable` semantics
- **AND** payload не подменяет это guessed `document_sync` / `completion` attribution

### Requirement: Superseded completion в `turn_wait` не становится orphaned до active registration (MUST)
Если same-file completion request уже вышел из per-file queue и вошёл в dispatcher `turn_wait`, но ещё не был зарегистрирован как active interactive completion, система MUST продолжать считать его частью same-file latest-wins/cancel lifecycle.

Для такого request система MUST:
- сохранять возможность bounded supersession/cancel до active registration;
- не требовать, чтобы stale request сначала стал active, чтобы затем его можно было остановить;
- не допускать seconds-scale inflight retention stale `turn_wait` request после того, как newer same-file completion или explicit cancel уже сделали его неактуальным;
- не превращать stranded `turn_wait` request в причину seconds-scale `service_future_created -> first poll` backlog для более нового same-file completion.

#### Scenario: Более новый same-file completion вытесняет older request, уже попавший в `turn_wait`
- **GIVEN** request `A` для одного `file_id` уже вышел из per-file queue и ожидает dispatcher turn
- **AND** request `A` ещё не зарегистрирован как active completion owner
- **AND** приходит более новый same-file completion request `B`
- **WHEN** сервер применяет latest-wins semantics
- **THEN** request `A` boundedly получает superseded/cancelled outcome без обязательного перехода в active state
- **AND** request `B` не накапливает seconds-scale pre-poll backlog из-за orphaned `turn_wait` request `A`

#### Scenario: Explicit cancel резолвит `turn_wait` request до active registration
- **GIVEN** completion request уже ожидает dispatcher turn
- **AND** клиент отправил `$/cancelRequest` для этого completion
- **WHEN** adapter и orchestrator обрабатывают cancel
- **THEN** stale request boundedly сворачивается ещё в `turn_wait` lifecycle
- **AND** request не публикует поздний user-facing completion ответ

### Requirement: Completion timeline truthfully отражает `turn_wait` lifecycle текущего request и stale contenders (MUST)
Если authoritative completion timeline публикует absolute `turn_wait` lifecycle текущего request, payload MUST позволять отделить:
- фактическое ожидание current request в `turn_wait`;
- stale contenders, которые всё ещё видимы в `phase=turn_wait`;
- immediate resolve current request без invented multi-second wait.

Сервер MUST NOT схлопывать multi-second current-request `turn_wait` stage в нулевую absolute lifecycle, если такой wait реально наблюдался.
Если current request резолвится immediately, но stale contender остаётся в `phase=turn_wait`, payload MUST показывать это как отдельный contender-state, а не как длительный current-request wait.

#### Scenario: Текущий request проходит `turn_wait` сразу, а stale contender остаётся в `phase=turn_wait`
- **GIVEN** current completion request получает dispatcher-ready outcome практически сразу
- **AND** authoritative contenders всё ещё содержат older same-file completion в `phase=turn_wait`
- **WHEN** оператор читает completion timeline
- **THEN** current-request `turn_wait` absolute lifecycle остаётся immediate
- **AND** stale `turn_wait` contender показывается отдельно через bounded contender fields
- **AND** payload не приписывает multi-second current-request wait только по возрасту stale contender

#### Scenario: Multi-second current `turn_wait` не схлопывается в нулевую absolute lifecycle
- **GIVEN** текущий completion request реально провёл multi-second время в `turn_wait`
- **WHEN** сервер сериализует authoritative completion timeline
- **THEN** absolute `turn_wait` lifecycle остаётся согласованным со stage duration в пределах bounded measurement tolerance
- **AND** payload не выдумывает immediate resolve/wake, если wait реально длился секунды

### Requirement: Same-file overlap gate ловит stranded pre-active `turn_wait` request (MUST)
Acceptance для completion overlap MUST включать сценарий, где older same-file completion теряет актуальность, пока он уже вышел из queue, но ещё не стал active owner.

Этот gate MUST:
- воспроизводить same-file overlap через live LSP path;
- fail-ить, если stale contender остаётся видимым в `phase=turn_wait` за пределами bounded supersession window;
- fail-ить, если новый same-file completion копит seconds-scale `service_future_created -> first poll` backlog из-за stranded pre-active predecessor;
- сохранять checked-in evidence, достаточную для различения pre-active `turn_wait` blind spot от stale active `response_build` retention.

#### Scenario: Representative overlap gate ловит stranded pre-active predecessor
- **GIVEN** live overlap profile на representative real module
- **AND** request `A` уже успел войти в `turn_wait`, но ещё не стал active owner
- **AND** request `B` для того же файла приходит после `A`
- **WHEN** gate измеряет same-file completion overlap
- **THEN** gate требует bounded terminal outcome для `A`
- **AND** gate требует, чтобы `B` достигал first poll без seconds-scale pre-poll backlog из-за stale `turn_wait` predecessor

### Requirement: Event-driven completion освобождает transport slot до длительного passive `turn_wait` (MUST)
На default event-driven completion path LSP request MUST NOT удерживать `tower-lsp` transport admission slot только потому, что request пассивно ждёт dispatcher turn или older same-file turn owner.

Перед таким wait система MUST:
- захватить request correlation и cancellation context, необходимые для normal completion response path;
- зафиксировать completion-owned handoff boundary, после которой passive wait больше не считается transport-slot retention;
- сохранить same-file latest-wins/cancel semantics для request, который ещё не начал heavy completion work.

#### Scenario: Current same-file completion ждёт older owner без seconds-scale pre-first-poll backlog
- **GIVEN** completion request `B` для файла приходит, пока older same-file request `A` ещё удерживает dispatcher turn
- **AND** `B` должен подождать release текущего owner, прежде чем начать heavy completion stages
- **WHEN** сервер принимает `B` на default event-driven path
- **THEN** transport admission slot освобождается до multi-second passive `turn_wait`
- **AND** authoritative trace не показывает seconds-scale `service_future_created -> first_poll` backlog только из-за ожидания turn для `B`
- **AND** `B` позже продолжает completion lifecycle по normal response path

#### Scenario: Explicit cancel останавливает completion после handoff, но до heavy work
- **GIVEN** completion request уже прошёл handoff boundary и ещё только пассивно ждёт dispatcher turn
- **AND** клиент отправляет `$/cancelRequest` для этого completion
- **WHEN** adapter и completion orchestrator обрабатывают cancel
- **THEN** request boundedly сворачивается без late publish user-facing completion ответа
- **AND** transport slot не удерживается до терминального завершения этого passive wait

### Requirement: Post-handoff completion сохраняет single-owner и exactly-once terminal semantics (MUST)
После completion handoff система MUST назначать ровно одного lifecycle owner, который владеет:
- `request_id` и correlation context для terminal response path;
- cancellation/shutdown cleanup;
- правом отправить не более одного terminal response или завершить request fail-closed, если transport уже недоступен.

Dispatcher MUST оставаться единственным authority для `latest-wins` и publishability. Post-handoff completion task MUST NOT самостоятельно становиться publishable в обход dispatcher/epoch checks.

#### Scenario: Cancel race не приводит к двойному terminal response
- **GIVEN** completion request уже передан post-handoff owner и ещё не начал heavy work
- **AND** почти одновременно приходят `$/cancelRequest` и wakeup/resolution для ожидания turn
- **WHEN** lifecycle owner и dispatcher обрабатывают эту гонку
- **THEN** для данного `request_id` наблюдается не более одного terminal outcome
- **AND** request не публикует поздний completion ответ после terminal cleanup

#### Scenario: Supersede race сохраняет latest-wins и exactly-once cleanup
- **GIVEN** older same-file completion уже передан post-handoff owner
- **AND** newer same-file completion supersedes older request до начала heavy work
- **WHEN** dispatcher и lifecycle owner обрабатывают supersede
- **THEN** older request получает bounded terminal cleanup ровно один раз
- **AND** newer request остаётся единственным publishable same-file completion

#### Scenario: Shutdown race завершает handoff owner fail-closed без late publish
- **GIVEN** completion request уже передан post-handoff owner
- **AND** server shutdown начинается до terminal completion response
- **WHEN** lifecycle owner обрабатывает shutdown
- **THEN** owner boundedly завершает cleanup без двойного terminal response
- **AND** после shutdown не появляется поздний publish user-facing completion ответа

### Requirement: Completion timeline отделяет off-transport wait от ingress backlog (MUST)
Если authoritative completion timeline публикует latency profile request, payload MUST позволять отделить:
- ingress backlog до handoff / admission;
- completion-owned wait после handoff;
- stale contenders, которые всё ещё видимы в `phase=turn_wait`.

Сервер MUST NOT объяснять multi-second off-transport wait через `service_future_created -> first_poll` или через handler-resident passive `turn_wait`, если transport slot уже освобождён.

#### Scenario: First poll bounded, а multi-second wait идёт после handoff
- **GIVEN** current completion request быстро проходит transport admission path
- **AND** затем request проводит multi-second время в passive wait за same-file turn owner
- **WHEN** оператор читает authoritative completion timeline
- **THEN** payload сохраняет bounded ingress attribution до handoff
- **AND** multi-second completion-owned wait показывается отдельно от `service_future_created -> first_poll`
- **AND** payload не маскирует off-transport wait под transport backlog

#### Scenario: Connected server ещё не поддерживает handoff-aware contract
- **GIVEN** connected server возвращает timeline старой версии без новых handoff-aware полей
- **WHEN** extension или operator читает payload
- **THEN** клиент не выдумывает off-transport wait attribution
- **AND** trustworthy semantics остаются ограничены реально присутствующими полями

### Requirement: Same-file overlap gate ловит completion `turn_wait` transport-slot retention (MUST)
Acceptance для same-file overlap MUST fail-ить не только на stranded contender или pre-first-poll backlog, но и на сценарий, где current completion request всё ещё проводит seconds-scale passive `turn_wait` внутри transport/handler path.

Этот gate MUST:
- воспроизводить same-file overlap через live LSP default path;
- fail-ить, если current request удерживает transport/handler path на multi-second passive `turn_wait`;
- fail-ить, если same-file overlap снова превращает completion lifecycle в ingress bottleneck;
- сохранять checked-in evidence, достаточную для различения ingress backlog, stale contender и off-transport wait.

#### Scenario: Representative overlap gate ловит inline `turn_wait`, удерживающий transport path
- **GIVEN** live same-file overlap profile на representative real module
- **AND** request `B` приходит, пока older same-file request `A` всё ещё удерживает turn
- **WHEN** gate измеряет latency profile `B`
- **THEN** gate требует bounded transport admission path для `B`
- **AND** gate завершает прогон ошибкой, если multi-second passive `turn_wait` всё ещё наблюдается внутри transport/handler path
- **AND** checked-in evidence позволяет отличить этот regression от stale pre-active contender

### Requirement: Completion timeline использует request-bound probe correlation key (MUST)
Если default VS Code completion path отправляет namespaced vendor correlation key `bslProbeId` для completion probe, authoritative LSP completion timeline MUST переносить этот opaque key в root-level field `client_probe_id` соответствующего per-request trace.

Derived extension surfaces, которые коррелируют local completion probes с server traces, MUST использовать этот key как primary correlation source. Timestamp/window эвристика MAY использоваться только как backward-compatible fallback для старых payload'ов и MUST оставаться fail-closed при отсутствии deterministic correlation.

Этот requirement дополняет существующий server-driven timeline contract и existing fail-closed правила для client-side ingress supplement; он MUST NOT заменяться guessed attribution по одним только local probe timings.

Реализация этого requirement MUST поднять authoritative completion timeline response version `17 -> 18` и contiguous contract baseline `contracts/lsp-completion-timeline/v14 -> v15`.

`client_probe_id` MUST оставаться только per-request correlation marker и MUST NOT становиться частью агрегированных observability counters, histograms или human-readable guessed summaries без authoritative trace.

#### Scenario: Overlap completion requests коррелируются без ambiguity
- **GIVEN** два completion probe для одного `uri` и одинакового `trigger_mode` перекрываются по времени
- **AND** оба request'а несут разные request-bound correlation keys
- **WHEN** сервер публикует authoritative completion timeline и extension строит incident bundle
- **THEN** каждый trace коррелируется с ровно одним probe по echoed correlation key
- **AND** incident bundle не получает `multiple_probe_candidates` только из-за overlap и близких timestamps

#### Scenario: Старый payload без echoed key деградирует fail-closed
- **GIVEN** extension получает timeline payload старой версии без request-bound correlation key
- **WHEN** derived surface пытается дополнить trace client-side ingress verdict
- **THEN** fallback MAY использовать legacy timestamp/window heuristic
- **AND** verdict не публикуется, если deterministic correlation всё равно не доказана

### Requirement: Completion Timeline panel остаётся quiet во время active completion (MUST)
VS Code `Completion Timeline` panel MUST NOT создавать дополнительный front-edge noise на default completion path.

Auto-refresh/polling MUST приостанавливаться или переходить в bounded backoff, пока есть active completion probes, и в течение короткого quiet window после их завершения. Incident export и panel rendering MUST по умолчанию использовать последний уже захваченный authoritative snapshot, а не форсировать fresh `bsl.getCompletionTimeline` в момент churn.

Explicit export command MAY делать fresh fetch только когда cached capture отсутствует; он MUST NOT обходить quiet policy в случае, когда authoritative snapshot уже захвачен webview path.

Manual refresh MAY оставаться доступным, но он MUST быть явным действием оператора, а не скрытым side effect active observability view.

#### Scenario: Видимая panel не мешает активному completion
- **GIVEN** `Completion Timeline` panel открыта и видима
- **AND** пользователь вызывает completion во время typing/load
- **WHEN** extension отслеживает active completion probes
- **THEN** panel не инициирует обычный polling `bsl.getCompletionTimeline`, пока active probe ещё не завершён
- **AND** incident/export path использует последний already-captured snapshot вместо forced fresh fetch

#### Scenario: Manual refresh остаётся explicit после quiet window
- **GIVEN** active completion probes уже завершились и quiet window истёк
- **WHEN** оператор вручную инициирует refresh panel
- **THEN** extension делает новый timeline fetch явным образом
- **AND** этот refresh не маскируется под background auto-polling

### Requirement: Same-version member-access completion не теряет didChange-produced exact-task visibility (MUST)
Если `didChange` уже запланировал exact type-index producer task для текущей версии файла, member-access completion на той же версии MUST наблюдать либо matching producer task, либо уже опубликованное `serve_only_ready` состояние до terminal decision request path.

Race-окно, в котором producer task завершился и исчез из registry раньше, чем readiness стала наблюдаемой, MUST NOT приводить к spurious `NoMatchingTask` для same-version `TriggerCharacter='.'` или `Invoked` request'ов.

Completed matching same-version task entry MUST оставаться observable/joinable до одного из bounded terminal cleanup events:
- `serve_only_ready` для той же версии стал наблюдаемым;
- task superseded новой версией;
- файл закрыт через `didClose`;
- сервер выполняет shutdown cleanup.

При этом request path MUST NOT сам создавать exact producer task, MUST NOT переходить на stale semantic fallback и MUST сохранять bounded fail-closed behaviour для genuine cold miss, wrong-version и deadline cases.

#### Scenario: Same-version `TriggerCharacter='.'` ждёт producer вместо spurious `NoMatchingTask`
- **GIVEN** `didChange` уже запланировал exact type-index producer task для версии `V`
- **AND** completion по `TriggerCharacter='.'` приходит для той же версии `V` до публикации observable `serve_only_ready`
- **WHEN** request path ожидает current-revision exact readiness
- **THEN** waiter видит matching producer task или готовый exact artifact для версии `V`
- **AND** request не завершается `NoMatchingTask` только из-за короткого race между producer completion и readiness publication

#### Scenario: Genuine cold miss остаётся bounded fail-closed
- **GIVEN** matching producer task для текущей версии не существует либо уже superseded другой версией
- **WHEN** member-access completion ждёт exact readiness в пределах bounded wait budget
- **THEN** request path остаётся fail-closed и bounded
- **AND** система не создаёт exact producer task на request path и не возвращает stale semantic substitute

#### Scenario: Completed matching task очищается только по bounded cleanup rules
- **GIVEN** exact producer task для текущей версии уже завершил compute, но matching same-version waiter ещё может обратиться к registry
- **WHEN** `serve_only_ready` ещё не наблюдаем и не произошло supersession, `didClose` или shutdown
- **THEN** completed task entry остаётся observable для same-version waiter
- **AND** cleanup не происходит преждевременно только из-за факта single-run completion

### Requirement: Auxiliary `documentSymbol` traffic не starving interactive semantic admission (MUST)
Система MUST рассматривать `textDocument/documentSymbol` как auxiliary IDE companion request и MUST изолировать его admission/execution path от interactive semantic запросов (`completion`, `hover`, `signatureHelp`, `definition`).

Изоляция MUST обеспечивать:
- outstanding `documentSymbol` refresh не задерживает первый `poll()` interactive запроса из-за strict current-version wait;
- auxiliary path не потребляет interactive reserve при наличии interactive waiters;
- same-file newer `documentSymbol` refresh MAY supersede older outstanding refresh, если старый ещё не принёс user-visible value;
- `documentSymbol` outcome (`current_ready`, `latest_ready`, `unavailable`, `superseded`) не влияет на strict current-revision contract interactive semantic ответов.

#### Scenario: Outline refresh не блокирует completion ingress
- **GIVEN** для того же файла одновременно идут `didChange`/`didSave` churn и refresh Outline через `textDocument/documentSymbol`
- **AND** пользователь запрашивает member-access completion
- **WHEN** сервер обрабатывает mixed load
- **THEN** completion получает first `poll()` без ожидания завершения outstanding `documentSymbol` current-version wait
- **AND** `documentSymbol` обслуживается как auxiliary outcome, а не как gate для interactive completion

#### Scenario: Более новый outline refresh supersede-ит старый
- **GIVEN** для одного `file_id` в очереди уже есть outstanding `documentSymbol` refresh
- **AND** приходит более новый `documentSymbol` refresh после следующего `didChange`
- **WHEN** сервер выбирает, какой refresh исполнять
- **THEN** older refresh может быть superseded в пользу newest refresh
- **AND** supersession фиксируется как явный auxiliary outcome

### Requirement: Mixed-load gate детерминированно ловит outline-induced starvation (MUST)
Система MUST иметь representative live gate, который прогоняет same-file real-module mixed load из:
- `didChange`/`didSave`;
- `textDocument/documentSymbol`;
- `textDocument/completion`.

Gate MUST собирать authoritative server-side evidence минимум по:
- completion `service_future_to_first_poll_wait_ms`;
- completion `transport_to_handler_wait_ms`;
- completion route/outcome;
- `documentSymbol` outcome class (`current_ready`, `latest_ready`, `unavailable`, `superseded`).

Gate MUST fail:
- если `p95(service_future_to_first_poll_wait_ms)` у measured completion samples выше `intellisense_v2_interactive_wait_budget_ms`;
- если любой measured completion sample имеет `service_future_to_first_poll_wait_ms > 4 * intellisense_v2_interactive_wait_budget_ms`;
- если measured completion sample становится ingress-dominant из-за concurrent auxiliary `documentSymbol` load.

#### Scenario: Representative gate падает при starvation от outline traffic
- **GIVEN** real-module mixed-load profile с active `documentSymbol` refresh и completion на том же файле
- **WHEN** auxiliary outline path снова начинает удерживать interactive completion до входа в handler
- **THEN** representative gate завершается ошибкой
- **AND** evidence указывает на concurrent outline outcome/load, а не маскирует regression как generic completion slowdown

### Requirement: Interactive completion admission изолирован от general LSP backlog до dispatch (MUST)
Система MUST изолировать `textDocument/completion` от unrelated general LSP traffic в окне между чтением request transport adapter'ом и dispatch в service pipeline.

Изоляция MUST обеспечивать:
- shared readiness/admission state MUST принадлежать одному scheduler owner; reader/producers MUST NOT вызывать `poll_ready()/call()` напрямую;
- completion request классифицируется и попадает в interactive admission queue до shared `poll_ready()` blocking для general traffic;
- general requests MUST NOT удерживать freshly-read completion request вне interactive admission queue только из-за общего readiness wait;
- completion-supporting document-sync notifications (`textDocument/didOpen`, `textDocument/didChange`, `textDocument/didSave`, `textDocument/didClose`), прочитанные transport adapter'ом до completion на том же transport path, MUST NOT теряться или застревать за unrelated general backlog так, чтобы последующий completion видел stale current revision;
- control traffic (`$/cancelRequest`, shutdown-related flow) MAY preempt queued completion admission;
- saturated completion spillover MUST оставаться bounded и fail-closed: older queued completion MAY завершаться pre-dispatch outcome `queue_rejected`, но single reader MUST NOT останавливаться так, чтобы поздний control traffic не был даже классифицирован;
- queued completion cancellation MUST сохранять existing exactly-once terminal semantics, MUST возвращать ровно один terminal response и MUST NOT допускать late publish после признанного cancel.

#### Scenario: General request burst не блокирует completion до dispatch
- **GIVEN** transport adapter уже читает burst general requests, включая `textDocument/documentSymbol`
- **AND** на том же transport path приходит новый completion request
- **WHEN** сервер выбирает, что dispatch-ить дальше
- **THEN** completion попадает в interactive admission queue без ожидания завершения general readiness path
- **AND** authoritative trace не показывает seconds-scale `adapter_to_dispatch_wait_ms` только из-за concurrent general backlog

#### Scenario: `didChange` handoff не теряется за unrelated general backlog на default path
- **GIVEN** transport adapter уже держит unrelated `textDocument/documentSymbol` backlog
- **AND** затем на том же transport path приходит `textDocument/didChange`, публикующий новую current revision
- **AND** после этого приходит completion request для того же документа
- **WHEN** сервер формирует first response для completion
- **THEN** `didChange` current-revision handoff достигает interactive admission path раньше completion result
- **AND** completion first response видит latest current revision, а не stale текст до `didChange`

#### Scenario: Queued completion отменяется до dispatch без late publish
- **GIVEN** completion request уже стоит в pre-dispatch queue
- **AND** до его dispatch приходит matching `$/cancelRequest`
- **WHEN** scheduler обрабатывает control lane
- **THEN** queued completion помечается cancelled до dispatch
- **AND** сервер возвращает ровно один terminal response с cancellation semantics `RequestCancelled`
- **AND** authoritative trace публикует outcome `cancelled` без выдуманных post-dispatch timestamps
- **AND** система сохраняет exactly-once terminal semantics без поздней публикации completion result

#### Scenario: Saturated completion spillover не прячет late cancel за reader stall
- **GIVEN** completion lane уже заполнен queued completion work
- **AND** bounded completion spillover тоже исчерпан
- **AND** затем на том же transport path приходит ещё один completion request, а сразу после него matching `$/cancelRequest`
- **WHEN** transport adapter применяет overflow policy до dispatch
- **THEN** older queued completion fail-closed завершается pre-dispatch outcome `queue_rejected` вместо блокировки single reader
- **AND** late `$/cancelRequest` всё ещё классифицируется и отменяет самый новый queued completion до dispatch
- **AND** transport path сохраняет exactly-once terminal semantics без late publish

### Requirement: Auxiliary LSP CPU work stays isolated from interactive transport/runtime loops (MUST)
CPU-heavy auxiliary LSP work, не являющаяся primary semantic body текущего interactive ответа, MUST выполняться через bounded blocking или эквивалентную isolated CPU boundary и MUST NOT выполняться inline на async runtime threads, которые обслуживают:
- transport read/write loops;
- admission и service scheduling;
- first polling service futures;
- completion handoff/output progression.

Этот contract MUST покрывать как минимум:
- documentSymbol ready-cache materialization и same-version outline refresh, инициированные document-sync path;
- parse/context derivation для auxiliary request path `bsl.getCurrentContext`, когда для ответа нужен полный parse текущего текста файла.

Auxiliary jobs MAY оставаться bounded, cancellable и coalesced, но MUST NOT вызывать seconds-scale `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` или `response_output_handoff_send_wait_ms` regressions для same-file interactive completion, если primary completion path уже hot/ready.

#### Scenario: Background outline materialization не выполняет symbol building inline на async runtime
- **GIVEN** document-sync worker уже завершил bounded parse для requested revision
- **WHEN** сервер materializes latest-ready outline cache для того же файла
- **THEN** CPU-heavy symbol derivation выполняется через bounded auxiliary CPU boundary
- **AND** newer same-file completion не теряет runtime progress только из-за этого auxiliary work

#### Scenario: `bsl.getCurrentContext` parse не starvation-ит concurrent completion
- **GIVEN** extension почти одновременно вызывает `bsl.getCurrentContext` и `textDocument/completion` для крупного модуля
- **AND** current-context request требует parse/context derivation
- **WHEN** сервер обслуживает оба запроса
- **THEN** current-context auxiliary CPU work не выполняется inline на async transport/runtime loop
- **AND** completion trace не получает seconds-scale ingress или output-handoff delay только из-за `bsl.getCurrentContext`

### Requirement: Representative mixed-load guard budgets truthful ingress and handoff seams (MUST)
Representative mixed-load regression coverage для completion MUST budget-ить truthful latency seams, которые остаются user-visible после probe/egress split, а не только legacy pre-dispatch ingress split.

Guard MUST как минимум:
- использовать same-file profile `didChange + didSave + documentSymbol burst + completion` на representative large-module fixture;
- собирать authoritative fields `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`;
- fail-ить, если auxiliary runtime work уводит trace в seconds-scale ingress или handoff backlog, даже если `adapter_to_dispatch_wait_ms` остаётся в бюджете;
- сохранять existing correctness checks для non-empty completion, fail-closed counters и `documentSymbol latest_ready` behavior.

#### Scenario: Truthful mixed-load gate ловит starvation, скрытую от legacy pre-dispatch split
- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** completion handler hot path уже ready или fast
- **WHEN** auxiliary outline/context work regression-ит и stall-ит transport ingress или completion handoff
- **THEN** representative gate завершается ошибкой по truthful `client_to_transport_wait_ms` или `response_output_handoff_send_wait_ms`
- **AND** regression не маскируется только потому, что `adapter_to_dispatch_wait_ms` остался в бюджете

### Requirement: Same-file save-triggered auxiliary churn does not regress current-revision readiness fast lane (MUST)
После того как current-revision handoff для requested revision уже зарегистрирован через `didOpen` или `didChange`, same-file `didSave`-triggered refresh и другой auxiliary same-file churn MAY продолжаться в фоне, но MUST NOT возвращать interactive completion к `prepare_timeout@wait_for_file_version`, если truthful transport seams уже остаются в интерактивном бюджете.

Для этого requirement readiness regression считается отдельным failure mode:
- bounded wait на `wait_for_file_version` не должен исчерпываться только потому, что newest same-file readiness все еще стоит позади save-triggered auxiliary backlog;
- healthy `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` MUST NOT использоваться как оправдание для `prepare_timeout@wait_for_file_version`;
- cold semantic/query-body latency после успешного readiness рассматривается отдельно и не считается объяснением readiness timeout.

#### Scenario: Same-file save refresh не держит newest completion в `wait_for_file_version`
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для requested revision `V`
- **AND** same-file `didSave` или другой auxiliary refresh запускает дополнительную background работу для того же файла
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness fast lane не деградирует в `prepare_timeout@wait_for_file_version` только из-за этого same-file auxiliary backlog
- **AND** completion либо получает current-revision first response, либо завершается по другой truthful причине, не связанной с post-handoff `wait_for_file_version` starvation

#### Scenario: Healthy truthful seams не маскируют readiness timeout
- **GIVEN** representative completion sample показывает `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` внутри интерактивного бюджета
- **WHEN** тот же sample все равно завершает prepare как `prepare_timeout@wait_for_file_version`
- **THEN** outcome считается current-revision readiness regression
- **AND** не считается допустимым bounded fail-closed поведением

### Requirement: Representative post-edit/save churn gate separates readiness regressions from cold query-body cost (MUST)
Representative real-module acceptance для current-revision completion MUST иметь отдельный post-edit/save churn profile, который проверяет readiness fast lane независимо от latency дальнейшего semantic/query-body execution.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + auxiliary same-file noise + completion` на representative large-module fixture;
- собирать truthful transport/readiness fields как минимум `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms`, `response_output_handoff_send_wait_ms` и `prepare_timeout` phase/cause;
- fail-ить, если measured sample получает `prepare_timeout@wait_for_file_version` при truthful transport seams внутри бюджета;
- report-ить cold `query_bundle_ir_query` / `collect` latency отдельным diagnostic bucket после успешного readiness, а не как оправдание readiness failure.

#### Scenario: Gate отдельно ловит readiness timeout и отдельно cold query-body
- **GIVEN** representative same-file post-edit/save churn profile на real module
- **AND** truthful transport seams measured sample остаются в бюджете
- **WHEN** один sample завершает prepare как `prepare_timeout@wait_for_file_version`, а другой sample успешно проходит readiness и тратит время в `query_bundle_ir_query`
- **THEN** gate завершается ошибкой из-за readiness timeout sample
- **AND** cold query-body latency отражается отдельным diagnostic signal, а не как причина acceptance failure по readiness contract

### Requirement: Immediate same-file post-edit/save completion window does not regress into `prepare_timeout` (MUST)
После того как same-file current-revision handoff уже зарегистрирован через `didChange` или `didSave`, первые interactive completion requests в immediate post-edit/save window MUST NOT завершаться `prepare_timeout` только потому, что fully prepared current-revision path ещё не стала наблюдаемой на request path, если truthful transport seams остаются в интерактивном бюджете.

Для этого requirement front-edge regression surface включает:
- `prepare_timeout@wait_for_file_version`;
- `prepare_timeout@snapshot_with_deps`, если timeout происходит в том же immediate post-edit/save window и не объясняется transport ingress/output backlog.

#### Scenario: Same-file front-edge completion не умирает на `wait_for_file_version`
- **GIVEN** `didChange` или `didSave` уже зарегистрировал same-file handoff для revision `V`
- **AND** IDE запрашивает completion почти сразу после этого handoff
- **WHEN** truthful transport seams остаются в интерактивном бюджете
- **THEN** completion не завершается `prepare_timeout@wait_for_file_version` только из-за front-edge readiness lag
- **AND** outcome либо остаётся bounded current-revision first response, либо завершается по другой truthful причине, не связанной с front-edge starvation

#### Scenario: Same-file front-edge completion не маскирует timeout на `snapshot_with_deps`
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** `wait_for_file_version` уже не объясняет timeout
- **WHEN** completion всё равно исчерпывает prepare budget на `snapshot_with_deps` в immediate post-edit/save window
- **THEN** такой outcome считается front-edge readiness regression
- **AND** не считается допустимым bounded fail-closed поведением

### Requirement: Representative front-edge gate separates immediate `prepare_timeout` regressions from cold `query_bundle_pool_wait` (MUST)
Representative real-module acceptance для current-revision completion MUST иметь отдельный immediate post-edit/save front-edge profile, который проверяет первые same-file completion samples сразу после handoff независимо от downstream cold query-body latency.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + immediate completion burst` на representative large-module fixture;
- собирать truthful transport/readiness fields как минимум `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms`, `response_output_handoff_send_wait_ms`, `fail_closed_cause` и `timeout_attribution.phase`;
- fail-ить на любом `prepare_timeout` в front-edge samples при healthy truthful transport seams;
- report-ить successful samples с cold `query_bundle_pool_wait` отдельным diagnostic bucket после успешного readiness.

#### Scenario: Gate валится на front-edge timeout и отдельно отражает downstream pool wait
- **GIVEN** representative immediate post-edit/save front-edge profile на real module
- **AND** truthful transport seams measured samples остаются в бюджете
- **WHEN** один sample завершается `prepare_timeout`, а другой sample успешно проходит readiness и тратит время в `query_bundle_pool_wait`
- **THEN** gate завершается ошибкой из-за front-edge `prepare_timeout`
- **AND** `query_bundle_pool_wait` отражается отдельным diagnostic signal, а не объяснением readiness failure

### Requirement: Immediate same-file front-edge completion does not regress into hidden `exact_deadline` (MUST)
После того как same-file current-revision handoff уже зарегистрирован через `didChange` или `didSave`, первые interactive completion requests в immediate post-edit/save window MUST NOT исчерпывать bounded `wait_exact_type_index` и затем завершаться generic fail-closed outcome только потому, что exact current-revision artifact ещё не стал наблюдаемым, если truthful transport seams остаются в интерактивном бюджете.

Для этого requirement:
- `wait_exact_type_index` exhaustion с `type_index_wait_outcome=deadline` в том же front-edge окне считается unresolved readiness regression;
- такой outcome не должен маскироваться под generic `missing_semantic_index` без explicit regression attribution.

#### Scenario: Front-edge exact wait deadline не маскируется как generic availability miss
- **GIVEN** `didChange` или `didSave` уже зарегистрировал same-file handoff для revision `V`
- **AND** completion request входит в immediate post-edit/save window почти сразу после handoff
- **WHEN** truthful transport seams остаются в интерактивном бюджете
- **THEN** completion не завершает front-edge path с hidden `wait_exact_type_index=deadline`
- **AND** operator-facing evidence не схлопывает такой regression в generic `missing_semantic_index` без отдельной attribution

### Requirement: Representative front-edge gate requires successful current-revision sample before separating cold `query_bundle_pool_wait` (MUST)
Representative real-module acceptance для current-revision completion MUST считать remediation незавершённой, если immediate post-edit/save front-edge profile не даёт ни одного successful current-revision sample, даже когда `prepare_timeout` уже устранён.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + immediate completion burst` на representative large-module fixture;
- fail-ить на любом front-edge `prepare_timeout` или hidden `exact_deadline` при healthy truthful transport seams;
- требовать как минимум один successful current-revision sample в measured front-edge window;
- report-ить cold `query_bundle_pool_wait` отдельным diagnostic bucket только для successful samples после readiness.

#### Scenario: Gate не проходит на all-fail-closed front-edge profile
- **GIVEN** representative immediate post-edit/save front-edge profile на real module
- **AND** truthful transport seams measured samples остаются в бюджете
- **WHEN** measured samples не содержат `prepare_timeout`, но все measured traces завершаются fail-closed до successful current-revision response
- **THEN** gate завершается ошибкой
- **AND** remediation не считается завершённой только на основании bounded fail-closed outcomes

### Requirement: Aged non-member current-revision completion does not block first response on exact re-probe (MUST)

Система MUST формировать aged non-member current-revision first response без blocking exact re-probe,
если exact не был уже доказан из prepared current-revision state.

Если non-member completion request уже использует `shadow_current_revision_fast_path`, current-revision
shadow/support state для requested revision уже подготовлен, а request вышел из immediate apply-age
window, first response MUST NOT синхронно re-probe-ить свежий current-revision snapshot только ради
повторной проверки exact readiness перед terminal decision.

В этом режиме request path:

- MAY возвращать exact только если exact readiness уже доказана из подготовленного current-revision state;
- MUST иначе переходить в bounded lightweight/no-IR current-revision path;
- MUST NOT возвращаться к effectively exact-only first-response поведению;
- MUST NOT получать seconds-scale stall или fail-closed `exact_deadline` только потому, что был сделан post-window exact re-probe.

#### Scenario: Aged non-member invoked completion уходит в bounded current-revision fallback без blocking exact re-probe

- **GIVEN** same-file invoked completion идёт через `shadow_current_revision_fast_path`
- **AND** request не является member-access
- **AND** current-revision shadow/support state для requested revision уже prepared
- **AND** request уже вышел из immediate apply-age window
- **AND** exact readiness не доказана из подготовленного состояния
- **WHEN** handler формирует first response
- **THEN** request не делает blocking exact re-probe как prereq terminal decision
- **AND** возвращает bounded truthful current-revision lightweight/no-IR response
- **AND** не регрессирует в `exact_deadline` только из-за post-window re-probe

### Requirement: Completion timeline truthfully covers blocking current-revision snapshot reacquisition (MUST)

Система MUST truthfully покрывать blocking current-revision snapshot reacquisition в authoritative
completion timeline.

Если completion request path всё ещё делает blocking current-revision snapshot reacquisition или
эквивалентный exact re-probe до terminal first-response decision, authoritative timeline MUST либо:

- явно публиковать эту работу как отдельный low-cardinality stage внутри `stages`, либо
- удерживать разницу между `total_duration_ms` и последним видимым stage end в пределах bounded capture overhead.

Authoritative trace MUST NOT приписывать доминирующую latency unrelated visible stage, если основная
часть request-path времени ушла в неатрибутированную blocking snapshot reacquisition.

#### Scenario: Blocking current-revision snapshot reacquisition не скрывается внутри uncovered handler gap

- **GIVEN** representative aged completion trace тратит заметное время на current-revision snapshot reacquisition до terminal decision
- **WHEN** сервер сериализует authoritative completion timeline
- **THEN** trace либо показывает dedicated low-cardinality stage для этой blocking work
- **OR** не оставляет seconds-scale gap между `total_duration_ms` и последним видимым stage end
- **AND** operator может отличить эту latency от `handler_prelude` и `query_bundle*`

### Requirement: `bsl.getCurrentContext` honors client latest-only generations with bounded supersession (MUST)

Server MUST honor bounded client latest-only generations for `bsl.getCurrentContext`.

Если client current-context surface передаёт bounded generation hints для `bsl.getCurrentContext`,
server MUST использовать их для bounded supersession/coalescing obsolete auxiliary work.

Для одного editor session backend:

- MUST NOT позволять obsolete older generations неограниченно накапливать independent expensive parse/context derivation;
- MUST supersede older generation до expensive parse/context derivation или коалесцировать её с эквивалентным newer work;
- MUST NOT делать obsolete response источником current context для newer generation;
- MAY по-прежнему возвращать bounded auxiliary response для superseded request, если это не нарушает newest-generation-wins semantics на client side.

#### Scenario: Cursor burst supersede-ит obsolete current-context work до expensive parse

- **GIVEN** extension отправляет несколько `bsl.getCurrentContext` requests одного editor session с монотонно растущими generation hints
- **AND** более новая generation становится известна серверу до завершения expensive parse для older request
- **WHEN** backend обслуживает этот burst
- **THEN** older request не доходит независимо до полного expensive parse/context derivation
- **AND** auxiliary path остаётся bounded по obsolete work
- **AND** newer generation остаётся единственным current candidate для client-visible context surface

### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного `textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`, `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как synthetic surrogate для direct save-fastlane bypass path;
- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что primary blocker это `apply_lag` / `wait_for_file_version`.

#### Scenario: didSave refresh экспортируется с dedicated save-cycle identity
- **GIVEN** пользователь сохраняет документ
- **WHEN** diagnostics runtime запускает refresh для `didSave`
- **THEN** система создаёт request-centric trace этого refresh
- **AND** trace содержит monotonic `save_cycle_sequence`
- **AND** trace можно получить через dedicated diagnostics save timeline surface
- **AND** trace не требует реконструкции из aggregate metrics

#### Scenario: operator-facing ordering двух save-cycle не зависит от diagnostics_generation
- **GIVEN** документ получает два `didSave` при одном и том же `requested_version`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** система показывает distinct `save_cycle_sequence` для каждого cycle
- **AND** trace остаётся truthful даже если `diagnostics_generation` не годится как save ordering key

#### Scenario: timeline объясняет stalled heavy follow-up request-centric причиной
- **GIVEN** `didSave` cycle уже дал `save_fastlane` first publish
- **AND** richer heavy follow-up ещё не published
- **WHEN** оператор читает diagnostics save timeline
- **THEN** trace показывает request-centric follow-up wait reason
- **AND** оператор может отличить apply-lag от semantic-work pending

#### Scenario: fastlane fallback публикует blocking queue wait отдельно от syntax query
- **GIVEN** `save_fastlane` first publish идёт через bounded blocking fallback path
- **WHEN** trace экспортируется в diagnostics save timeline
- **THEN** queue wait перед parse фиксируется отдельно от `syntax_diagnostics_query_ms`
- **AND** оператор может отличить queue wait от actual syntax query work

### Requirement: save_fastlane и idle_heavy группируются в один didSave refresh cycle (MUST)
Если один `didSave` запускает сначала `save_fastlane`, а затем `idle_heavy`, система MUST экспортировать их как
части одного save refresh cycle, а не как два несвязанных trace.

Trace MUST:

- явно различать `first_publish_profile` и optional `followup_profile`;
- сохранять порядок publish событий внутри одного cycle;
- не позволять follow-up другого `didSave` быть ошибочно приписанным предыдущему cycle.

#### Scenario: fastlane и heavy follow-up видны как один refresh cycle
- **GIVEN** для одного `didSave` сначала публикуется `save_fastlane`, а затем `idle_heavy`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** он видит один save refresh cycle
- **AND** внутри него first publish и follow-up отображаются отдельно, но с общим cycle identity

#### Scenario: Новый didSave не прилипает к предыдущему refresh cycle
- **GIVEN** для документа идут два последовательных `didSave`
- **WHEN** follow-up publish второго save завершается позже первого
- **THEN** diagnostics save timeline не смешивает publish события разных save cycle
- **AND** каждый cycle остаётся request-centric и truthful для своей `version/save_cycle_sequence`

### Requirement: didSave save_fastlane публикует bounded same-version first refresh (MUST)
`save_fastlane` MUST давать bounded same-version first publish после `didSave`, даже если applied-analysis snapshot ещё не готов.

Если `save_fastlane` падает в syntax-only shadow fallback, этот path MUST:

- не ждать shared bounded interactive queue как primary gating step;
- не публиковать diagnostics от older revision;
- оставаться supersession-aware для newer `didSave`.

#### Scenario: save_fastlane shadow fallback bypass-ит shared queue starvation
- **GIVEN** shared interactive blocking queue насыщена другой работой
- **AND** `didSave` first publish вынужден идти через shadow parse fallback
- **WHEN** diagnostics runtime публикует `save_fastlane` first refresh
- **THEN** first publish не тратит seconds-scale latency только на shared queue wait
- **AND** trace не маскирует bypass synthetic `blocking_queue_wait_ms`

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker
