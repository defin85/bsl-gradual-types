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

На `didChange` система MUST пытаться обновлять parse state инкрементально через старое дерево и
edit chain.

Если incremental path не может быть применен корректно, система MUST детерминированно переходить
на full parse для той же exact target revision и фиксировать причину fallback в observability.

Для same-file burst revisions система MUST оставаться latest-wins:

- obsolete intermediate same-file revisions MAY быть coalesced до parse/materialization;
- система MUST NOT materialize ready snapshot для obsolete intermediate revision, если уже известен
  newer exact same-file target;
- система MUST NOT ухудшать exactness: materialized ready snapshot по-прежнему обязан совпадать с
  latest exact target revision/text hash.

#### Scenario: same-file burst coalesces obsolete revisions before parse starts

- **GIVEN** для одного `file_id` приходят `didChange` revisions `V`, `V+1`, `V+2` в пределах одного
  burst
- **AND** older ready-snapshot work ещё не начал blocking parse для `V`
- **WHEN** runtime prepares background ready-snapshot production
- **THEN** older target revisions MAY быть coalesced away before parse starts
- **AND** blocking parse starts only for the latest exact target revision available at that moment
- **AND** obsolete intermediate revisions do not materialize ready snapshots

#### Scenario: newer exact target suppresses stale materialization after older parse finished

- **GIVEN** background ready-snapshot production already parsed exact revision `V`
- **AND** before materialization/install the same file receives newer revision `V+1`
- **WHEN** the producer re-checks latest exact target before publishing ready artifacts
- **THEN** the producer skips stale materialization for `V`
- **AND** retargets to `V+1` instead of publishing obsolete exact artifacts

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
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version
`25`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через
`workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской
реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline
`contracts/lsp-completion-timeline/v22`, синхронизированный с текущим authoritative payload и его
bounded field-set.

`v25` MUST сохранять additive `v24` ingress/query-body/flush-aware/output-egress semantics,
включая grouped `query_bundle*` taxonomy, `response_sent_at_ms`, existing `response_output_*`
milestones и `response_flush_completed_at_ms`.

Контракт `v25` MUST включать:

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

Если `server_edge_details` присутствует, additive `v25` pre-dispatch decomposition MAY
включать:

- `adapter_read_started_at_ms`;
- `adapter_parse_completed_at_ms`;
- `read_loop_wait_reason`;
- `read_loop_wait_ms`;
- `pending_completion_spillover_depth`;
- `pending_general_request_staged`;
- `admission_try_enqueue_at_ms`;
- `admission_lane`;
- `admission_lane_depth_before`;
- `admission_lane_depth_after`;
- `admission_enqueue_outcome`;
- `admission_spillover_outcome`;
- `admission_enqueued_at_ms`;
- `admission_queue_wait_ms`;
- `scheduler_woke_at_ms`;
- `scheduler_poll_ready_entered_at_ms`;
- `scheduler_poll_ready_resolved_at_ms`;
- `scheduler_poll_ready_wait_ms`;
- `scheduler_dequeued_at_ms`;
- `completion_barrier_active_at_dequeue`;
- `completion_barrier_generation`;
- `completion_barrier_owner_method`;
- `completion_barrier_owner_uri`;
- `completion_barrier_owner_version`;
- `completion_barrier_wait_ms`;
- `scheduler_service_call_started_at_ms`;
- `scheduler_service_call_returned_at_ms`;
- `scheduler_service_call_sync_exec_ms`;
- `doc_sync_first_poll_exec_ms`;
- `doc_sync_first_poll_outcome`;
- `doc_sync_first_poll_method`;
- `doc_sync_first_poll_uri`;
- `doc_sync_first_poll_version`;
- `same_file_ingress_token_required_version`;
- `same_file_ingress_token_published_at_ms`;
- `same_file_ingress_token_source`;
- `same_file_ingress_token_wait_ms`;
- `scheduler_ready_to_dispatch_wait_ms`.

`read_loop_wait_reason` MUST использовать только bounded vocabulary:

- `completion_lane_space`;
- `general_lane_space`;
- `none`.

Если `read_loop_wait_reason` присутствует и не равен `none`, payload MUST включать и
`read_loop_wait_ms`.

Если `pending_completion_spillover_depth` присутствует, payload MUST отражать queue depth на
момент reader-side wait, а не post-facto агрегированную оценку.

Если `admission_lane_depth_before` или `admission_lane_depth_after` присутствуют, они MUST
описывать depth именно того admission lane, в который пытались enqueue текущий request.

`admission_lane` MUST использовать только bounded vocabulary:

- `control`;
- `interactive_completion`;
- `document_sync_ingress`;
- `general`.

Если `admission_enqueued_at_ms` присутствует, payload MUST включать и
`admission_queue_wait_ms`.

Если `scheduler_poll_ready_resolved_at_ms` присутствует, payload MUST включать и
`scheduler_poll_ready_wait_ms`, и `scheduler_ready_to_dispatch_wait_ms`.

Если `completion_barrier_active_at_dequeue=true`, payload SHOULD публиковать и
`completion_barrier_owner_method`; если owner относится к file-scoped document-sync path, payload
SHOULD публиковать и `completion_barrier_owner_uri`, и `completion_barrier_owner_version`.

Если `same_file_ingress_token_published_at_ms` присутствует, payload MUST включать и
`same_file_ingress_token_required_version`, и `same_file_ingress_token_wait_ms`.

Если `same_file_ingress_token_source` присутствует, payload MUST использовать bounded vocabulary:

- `did_open`;
- `did_change`;
- `did_save`;
- `did_close`;
- `other`.

Если additive `v25` admission split присутствует, compatibility field
`adapter_to_dispatch_wait_ms` MUST сохранять umbrella semantics для полного server-side интервала
между `adapter_read_at_ms` и earliest dispatch boundary.

Если additive `v25` admission split присутствует полностью, сумма
`admission_queue_wait_ms + scheduler_poll_ready_wait_ms + completion_barrier_wait_ms + same_file_ingress_token_wait_ms + scheduler_ready_to_dispatch_wait_ms`
MUST совпадать с `adapter_to_dispatch_wait_ms`.

#### Scenario: `v25` payload раскладывает local reader wait и `adapter_read -> dispatch`

- **GIVEN** completion request сначала столкнулся с reader-side wait из-за local spillover или
  затем уже был задержан до dispatch в service pipeline
- **WHEN** оператор читает `server_edge_details`
- **THEN** payload может публиковать `read_loop_wait_reason`, `read_loop_wait_ms`,
  `pending_completion_spillover_depth`, `admission_lane`, `admission_enqueued_at_ms`,
  `admission_queue_wait_ms`, `scheduler_poll_ready_resolved_at_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_required_version`, `same_file_ingress_token_published_at_ms`,
  `same_file_ingress_token_wait_ms` и `scheduler_ready_to_dispatch_wait_ms`
- **AND** `adapter_to_dispatch_wait_ms` остаётся compatibility umbrella для всего
  `adapter_read -> dispatch` окна

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v25`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v22` совпадает по bounded field-set с runtime
  payload
- **AND** policy/verification scripts валидируют именно `v25/v22`, а не более старую версию

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
Derived verdicts для `Completion Timeline` panel, clipboard и связанных extension projections MUST
строиться только из уже имеющихся bounded latency fields и MUST NOT маркировать trace как
ingress-bottleneck, если соответствующая ingress задержка отсутствует.

Derived verdict layer MUST:

- использовать bounded waits `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_wait_ms`, `adapter_to_dispatch_wait_ms`,
  `transport_to_method_wait_ms`, `method_prelude_exec_ms` и, при наличии deterministic
  correlation в downstream consumer, `client_to_transport_wait_ms`;
- строить ingress verdict только при положительной доминирующей задержке;
- различать как минимум `reader_backpressure_dominant`, `admission_queue_dominant`,
  `scheduler_poll_ready_dominant`, `completion_barrier_dominant`,
  `same_file_ingress_token_dominant`,
  `adapter_before_dispatch_dominant`, `server_before_method_entry_dominant` и
  `handler_prelude_dominant`;
- использовать `adapter_before_dispatch_dominant` как backward-compatible umbrella verdict только
  если finer `v25` admission split отсутствует;
- MAY различать `client_before_transport_dominant`, только если deterministic correlation уже
  доказала положительный wait до самой ранней authoritative server ingress boundary, local
  `read_loop_wait_ms` отсутствует или не доминирует, и server-side `v25` admission buckets не
  объясняют задержку;
- не выводить generic ingress verdict только потому, что `0 >= 0` или потому что одна из
  задержек отсутствует.

#### Scenario: Reader-side spillover dominates before dispatch

- **GIVEN** completion trace имеет положительный `read_loop_wait_ms`, вызванный
  `read_loop_wait_reason=completion_lane_space`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `reader_backpressure_dominant`
- **AND** trace не получает verdict `client_before_transport_dominant`

#### Scenario: Queue residence доминирует над shared readiness и handler prelude

- **GIVEN** completion trace имеет положительный `admission_queue_wait_ms`, который доминирует
  над `scheduler_poll_ready_wait_ms`, `same_file_ingress_token_wait_ms`,
  `transport_to_method_wait_ms` и `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `admission_queue_dominant`
- **AND** trace не получает verdict `client_before_transport_dominant`

#### Scenario: Shared readiness доминирует над queue residence

- **GIVEN** completion trace имеет положительный `scheduler_poll_ready_wait_ms`, который
  доминирует над `admission_queue_wait_ms`, `transport_to_method_wait_ms` и
  `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `scheduler_poll_ready_dominant`
- **AND** trace не деградирует в coarse `adapter_before_dispatch_dominant`, если `v25`
  admission split уже присутствует

#### Scenario: Completion barrier dominates and the owner stays attributable

- **GIVEN** completion trace имеет положительный `completion_barrier_wait_ms`, который
  доминирует над `admission_queue_wait_ms`, `scheduler_poll_ready_wait_ms` и
  `same_file_ingress_token_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `completion_barrier_dominant`
- **AND** authoritative payload сохраняет barrier owner attribution, если она была доступна на
  server side

#### Scenario: Server-side admission split suppresses false client ingress blame

- **GIVEN** request summary имеет deterministic probe correlation
- **AND** authoritative payload содержит положительный `admission_queue_wait_ms` или
  `scheduler_poll_ready_wait_ms` или `read_loop_wait_ms` или `same_file_ingress_token_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает verdict `client_before_transport_dominant`
- **AND** projection остаётся fail-closed по client-side supplement

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
Система MUST изолировать `textDocument/completion` от unrelated general LSP traffic в окне между
чтением request transport adapter'ом и dispatch в service pipeline.

Изоляция MUST обеспечивать:

- shared readiness/admission state MUST принадлежать одному scheduler owner; reader/producers MUST
  NOT вызывать `poll_ready()/call()` напрямую;
- completion request классифицируется и попадает в interactive admission queue до shared
  readiness blocking для general traffic;
- general requests MUST NOT удерживать freshly-read completion request вне interactive admission
  queue только из-за общего readiness wait;
- completion-supporting document-sync notifications
  (`textDocument/didOpen`, `textDocument/didChange`, `textDocument/didSave`,
  `textDocument/didClose`) MUST публиковать same-file ingress ownership/token через
  per-file owner, который применяет raw document ordering для этого файла и делает latest
  handoff observable до того, как later completion для того же файла зависит от него;
- same-file ingress token MUST публиковаться только после регистрации current-revision handoff
  для соответствующего `(file_id, version)`, а не на более ранней dispatcher-event boundary;
- once the relevant same-file ingress token is already published, unrelated same-priority work для
  других файлов MUST NOT удерживать later completion first response только из-за shared FIFO
  residence;
- control traffic (`$/cancelRequest`, shutdown-related flow) MAY preempt queued completion
  admission;
- saturated completion spillover MUST оставаться bounded и fail-closed: older queued completion MAY
  завершаться pre-dispatch outcome `queue_rejected`, но transport runtime MUST NOT деградировать в
  reader stall, который мешает позднему control traffic даже быть классифицированным;
- queued completion cancellation MUST сохранять existing exactly-once terminal semantics, MUST
  возвращать ровно один terminal response и MUST NOT допускать late publish после признанного
  cancel.

#### Scenario: Same-file ingress token делает completion независимым от unrelated same-priority FIFO

- **GIVEN** transport runtime уже держит queued work для других файлов
- **AND** для файла `F` same-file `didChange` или `didSave` уже опубликовал актуальный ingress
  token
- **AND** затем приходит completion request для того же файла `F`
- **WHEN** сервер формирует first response для completion
- **THEN** completion зависит от ingress token файла `F`, а не от unrelated same-priority FIFO
  residence
- **AND** first response не сидит seconds-scale только потому, что раньше были прочитаны
  unrelated document-sync requests для других файлов

#### Scenario: Dispatcher event не считается same-file ingress token publication

- **GIVEN** `didChange` для файла `F` уже был отправлен в completion dispatcher
- **AND** current-revision handoff для `(F, version)` ещё не зарегистрирован
- **WHEN** оператор читает authoritative trace
- **THEN** payload не считает same-file ingress token опубликованным
- **AND** later completion для файла `F` не может считаться wait-free только по факту раннего
  dispatcher event

#### Scenario: Queued completion отменяется до dispatch без late publish

- **GIVEN** completion request уже стоит в pre-dispatch queue
- **AND** до его dispatch приходит matching `$/cancelRequest`
- **WHEN** scheduler обрабатывает control lane
- **THEN** queued completion помечается cancelled до dispatch
- **AND** сервер возвращает ровно один terminal response с cancellation semantics
  `RequestCancelled`
- **AND** authoritative trace публикует outcome `cancelled` без выдуманных post-dispatch
  timestamps

### Requirement: Auxiliary LSP CPU work stays isolated from interactive transport/runtime loops (MUST)
CPU-heavy auxiliary LSP work, не являющаяся primary semantic body текущего interactive ответа, MUST выполняться через bounded blocking или эквивалентную isolated CPU boundary и MUST NOT выполняться inline на async runtime threads, которые обслуживают:
- transport read/write loops;
- admission и service scheduling;
- first polling service futures;
- completion handoff/output progression.

Этот contract MUST покрывать как минимум:
- documentSymbol ready-cache materialization и same-version outline refresh, инициированные document-sync path;
- parse/context derivation для auxiliary request path `bsl.getCurrentContext`, когда для ответа нужен полный parse текущего текста файла.

Для `bsl.getCurrentContext` same-file same-revision/text bursts MUST дополнительно:
- broker-иться до входа в blocking CPU boundary;
- допускать не более одного leader parse/context derivation на эквивалентный key;
- не заставлять follower requests получать independent blocking CPU permits только ради ожидания leader parse;
- завершаться через shared async wait или bounded empty outcome при supersession/budget exhaustion.

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

#### Scenario: Current-context burst делит один leader parse вместо нескольких blocking holders
- **GIVEN** несколько same-file `bsl.getCurrentContext` requests попадают на один и тот же current document text без ready snapshot
- **WHEN** первый request начинает parse/context derivation
- **THEN** сервер допускает только один leader parse для этого key
- **AND** follower requests не получают отдельные blocking CPU permits только ради ожидания leader parse
- **AND** parse fan-out остаётся bounded одним leader parse

### Requirement: Representative mixed-load guard budgets truthful ingress and handoff seams (MUST)
Representative mixed-load regression coverage для completion MUST budget-ить truthful latency
seams, которые остаются user-visible после `v25` admission decomposition, а не только legacy
pre-dispatch ingress split.

Guard MUST как минимум:

- использовать same-file profile `didChange + didSave + documentSymbol burst + completion` на
  representative large-module fixture;
- собирать authoritative fields `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_wait_ms`, `client_to_transport_wait_ms`,
  `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`;
- fail-ить, если same-file completion after the relevant ingress token is already published всё
  равно получает seconds-scale `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms` или
  `same_file_ingress_token_wait_ms`;
- fail-ить, если regression снова маскируется как client-side ingress, когда authoritative
  server-side `v25` admission split уже объясняет задержку;
- сохранять existing correctness checks для non-empty completion, fail-closed counters и
  `documentSymbol latest_ready` behavior.

#### Scenario: Representative gate ловит same-file residual after ready ingress token without bucket shift

- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** relevant same-file ingress token уже опубликован до measured completion
- **WHEN** measured completion sample всё ещё проводит seconds-scale время в
  `read_loop_wait_ms`, `admission_queue_wait_ms`, `scheduler_poll_ready_wait_ms`,
  `completion_barrier_wait_ms` или `same_file_ingress_token_wait_ms`
- **THEN** gate завершается ошибкой
- **AND** regression не маскируется под generic client ingress или cold query-body cost

#### Scenario: Representative evidence keeps a correlation slice for the worst outlier

- **GIVEN** representative mixed-load profile на крупном модуле уже поймал worst completion outlier
- **WHEN** оператор читает checked-in evidence
- **THEN** evidence сохраняет хотя бы один correlation slice с active same-file freshness pressure
  when present
- **AND** этот slice может включать barrier owner, required token version, current published token
  version/source и timestamps, достаточные чтобы сопоставить outlier с overlapping didChange train

### Requirement: Same-file save-triggered auxiliary churn does not regress current-revision readiness fast lane (MUST)
После того как current-revision handoff для requested revision уже зарегистрирован через `didOpen` или `didChange`, same-file `didSave`-triggered refresh и другой auxiliary same-file churn MAY продолжаться в фоне, но MUST NOT возвращать interactive completion к `prepare_timeout@wait_for_file_version`, если truthful transport seams уже остаются в интерактивном бюджете.

Для этого requirement readiness regression считается отдельным failure mode:
- bounded wait на `wait_for_file_version` не должен исчерпываться только потому, что newest same-file readiness все еще стоит позади save-triggered auxiliary backlog;
- readiness waiter registration MUST становиться observable без seconds-scale residence в generic background writer/runtime FIFO до самого факта passive waiting;
- passive readiness waiting MUST NOT требовать дополнительных blocking CPU permits только ради наблюдения за requested revision;
- healthy `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` MUST NOT использоваться как оправдание для `prepare_timeout@wait_for_file_version`;
- cold semantic/query-body latency после успешного readiness рассматривается отдельно и не считается объяснением readiness timeout.

#### Scenario: Same-file save refresh не держит newest completion в `wait_for_file_version`
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для requested revision `V`
- **AND** same-file `didSave` или другой auxiliary refresh запускает дополнительную background работу для того же файла
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness fast lane не деградирует в `prepare_timeout@wait_for_file_version` только из-за этого same-file auxiliary backlog
- **AND** completion либо получает current-revision first response, либо завершается по другой truthful причине, не связанной с post-handoff `wait_for_file_version` starvation

#### Scenario: Waiter registration не сидит за unrelated apply backlog перед passive wait
- **GIVEN** writer/runtime уже обрабатывает unrelated apply backlog для того же или другого файла
- **AND** интерактивный completion request должен дождаться requested revision `V`
- **WHEN** request переходит к readiness observation
- **THEN** request становится passive waiter без seconds-scale residency в generic background FIFO до регистрации wait
- **AND** дальнейшая latency truthfully атрибутируется либо actual apply lag, либо другой readiness cause, а не raw registration backlog

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
- MUST prefer exact ready parse snapshot текущей revision, когда он уже доступен;
- MUST NOT запускать independent parse followers для того же same-file same-revision/text key, если leader parse уже существует;
- MUST NOT делать obsolete response источником current context для newer generation;
- MAY по-прежнему возвращать bounded auxiliary response для superseded request, если это не нарушает newest-generation-wins semantics на client side;
- MAY завершать superseded или over-budget follower пустым response, пока leader продолжает прогрев reusable parse artifact.

#### Scenario: Cursor burst supersede-ит obsolete current-context work до expensive parse

- **GIVEN** extension отправляет несколько `bsl.getCurrentContext` requests одного editor session с монотонно растущими generation hints
- **AND** более новая generation становится известна серверу до завершения expensive parse для older request
- **WHEN** backend обслуживает этот burst
- **THEN** older request не доходит независимо до полного expensive parse/context derivation
- **AND** auxiliary path остаётся bounded по obsolete work
- **AND** newer generation остаётся единственным current candidate для client-visible context surface

#### Scenario: Same-revision burst коалесцируется за одним leader parse
- **GIVEN** extension отправляет несколько current-context requests для одной и той же revision/text до появления ready snapshot
- **WHEN** backend уже запустил leader parse для самого нового запроса
- **THEN** остальные эквивалентные запросы не запускают independent expensive parse/context derivation
- **AND** либо ждут shared result bounded образом, либо получают empty response при supersession/budget exhaustion
- **AND** newest-generation-wins semantics остаётся сохранённой

### Requirement: didSave diagnostics публикует request-centric save refresh timeline (MUST)
Система MUST публиковать bounded authoritative trace для каждого diagnostics refresh, инициированного
`textDocument/didSave`.

Этот trace MUST:

- быть server-authored;
- быть request-centric, а не derived из cumulative metrics;
- содержать `uri`, `requested_version`, `save_cycle_sequence`, `diagnostics_generation`,
  `trigger=did_save`;
- фиксировать bounded stage/runtime facts, достаточные для разбора first publish и heavy
  follow-up;
- не содержать raw document text, snippets или high-cardinality payload.

Дополнительно trace MUST:

- не создавать второй trace identity для уже terminal `(requested_version, save_cycle_sequence)`;
- не заставлять operator-facing cycle ordering выводиться из `diagnostics_generation`, если у двух
  save-cycle совпадает `requested_version`;
- публиковать `blocking_queue_wait_ms` только как factual wait перед shared blocking gate, а не как
  synthetic surrogate для direct save-fastlane bypass path;
- различать `save_fastlane` first publish и heavy follow-up stall;
- не оставлять active heavy follow-up в состоянии просто `pending`, если сервер уже знает, что
  primary blocker это `apply_lag` / `wait_for_file_version`;
- публиковать canonical low-cardinality outcome для zero-budget `ready_artifacts` probe;
- публиковать canonical low-cardinality outcome для bounded-wait `ready_artifacts` probe, если
  такой probe был выполнен;
- публиковать branch-selection context, достаточный чтобы оператор видел:
  - был ли `shadow_state` доступен в момент выбора ветки;
  - существовала ли same-version ready-snapshot task и в каком canonical task state она была;
- публиковать эти новые поля через additive versioned contract, где older payload versions
  деградируют явно как `unavailable_by_design`, а не silently.

#### Scenario: zero-budget ready-snapshot miss explains why shadow-state won
- **GIVEN** `didSave` cycle already completed `save_fastlane`
- **AND** exact same-version ready parse snapshot не был выбран для `idle_heavy`
- **WHEN** оператор читает diagnostics save timeline
- **THEN** timeline показывает explicit outcome zero-budget ready-snapshot probe
- **AND** timeline показывает, был ли доступен `shadow_state`
- **AND** timeline показывает canonical ready-snapshot task state вместо неявного `None`

#### Scenario: older timeline payload degrades explicitly
- **GIVEN** consumer читает diagnostics save timeline payload более старой contract version
- **WHEN** в этой версии ещё нет ready-snapshot miss attribution fields
- **THEN** consumer маркирует их как `unavailable_by_design`
- **AND** оператор не принимает отсутствие поля за отсутствие события

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

### Requirement: LSP publishes authoritative file-scoped snapshot readiness status (MUST)

LSP MUST provide an authoritative live snapshot-readiness contract for open documents through:

- custom request `bsl/getSnapshotStatus`
- custom notification `bsl/snapshotStatus`

The contract MUST stay file-scoped and MUST NOT be reconstructed from diagnostics save timeline,
completion timeline, or cumulative observability metrics.

The request/notification payload MUST use bounded low-cardinality fields including:

- `schemaVersion`
- `uri`
- `requestedVersion`
- `readyVersion`
- `state` with bounded vocabulary `idle | building | ready | stale | shadow_only | failed`
- `exact`
- `taskState`
- optional coarse `phase`
- optional `trigger`
- `updatedAtMs`
- optional bounded `fallbackReason`

The payload state vocabulary MUST use these meanings consistently:

- `idle`: no exact-ready, no degraded current answer, and no matching rebuild currently surfaced;
- `building`: a matching rebuild is in flight and exact readiness is not available yet;
- `ready`: exact-ready artifacts exist for the requested revision;
- `stale`: a ready artifact exists but is older than the requested revision;
- `shadow_only`: the current answer is based on `shadow_state` rather than exact ready artifacts;
- `failed`: the last attempted rebuild ended in an explicit error and the server cannot truthfully
  claim `ready` or `shadow_only`.

Notification updates MUST be coalesced per URI and MUST NOT emit unbounded micro-step noise for
every internal transition. Request fetch remains the hydrate/manual-read path.

For the same URI, `updatedAtMs` MUST be monotonic and clients MUST ignore an older update once they
have already observed a newer one.

#### Scenario: Same-version worker in flight reports building state
- **GIVEN** a matching same-version ready-snapshot worker is still in flight for an open document
- **WHEN** the client requests `bsl/getSnapshotStatus` for that document
- **THEN** the server returns `state=building`
- **AND** the payload stays truthful about the in-flight task state instead of claiming ready

#### Scenario: Exact ready snapshot reports exact-ready state
- **GIVEN** the requested document revision already has a matching ready snapshot
- **WHEN** the server serves snapshot readiness for that document
- **THEN** the payload reports `state=ready`
- **AND** `exact=true`
- **AND** `readyVersion` matches `requestedVersion`

#### Scenario: `shadow_only` fallback remains distinct from ready
- **GIVEN** the server can answer the current document only from `shadow_state` rather than exact
  ready snapshot artifacts
- **WHEN** snapshot readiness is reported
- **THEN** the payload reports `state=shadow_only`
- **AND** the payload does not claim exact readiness

#### Scenario: Live transition publishes coalesced notification
- **GIVEN** a document transitions from `building` to exact `ready`
- **WHEN** the server emits snapshot-readiness live updates
- **THEN** the client can observe the state change through `bsl/snapshotStatus`
- **AND** the server does not require timeline polling to surface that transition

#### Scenario: Older notification for the same URI is safely ignored by the client
- **GIVEN** the server has already emitted a newer snapshot-status update for a URI
- **WHEN** an older update for that same URI is delivered later
- **THEN** the client can distinguish it via `updatedAtMs`
- **AND** the older update does not overwrite the newer state

### Requirement: Warm non-member completion reuses immutable deps-scoped candidate catalogs (MUST)

Warm non-member completion MUST reuse immutable deps-scoped candidate catalogs for candidate families whose semantic content depends on deps/settings snapshot rather than on the current cursor-local revision state.

Для таких immutable families система MUST:

- prebuild or reuse a deps/settings-scoped catalog (or semantically equivalent immutable snapshot artifact);
- avoid rebuilding the full family on every warm non-member request under the same deps/settings snapshot;
- apply prefix-aware filtering before full `Candidate` materialization when a usable request prefix is already known;
- preserve existing source-priority and ranking semantics for the surviving candidates.

При этом система MUST keep revision/context-sensitive sources separate:

- local symbols;
- contextual implicit symbols;
- module routines and other cursor-local sources.

#### Scenario: Warm non-member completion не rebuild-ит immutable deps-wide catalogs на каждый request
- **GIVEN** deps/settings snapshot не менялся между двумя warm non-member completion requests
- **AND** request path уже имеет current-revision state для файла
- **WHEN** IDE запрашивает non-member completion повторно
- **THEN** immutable deps-scoped families переиспользуются из snapshot-scoped catalog вместо полного rebuild
- **AND** warm collect latency не доминируется повторной materialization тех же global functions / repository types / metadata items

#### Scenario: Prefix-aware filtering materialize-ит только нужный subset immutable catalog
- **GIVEN** non-member completion request содержит usable prefix
- **AND** immutable deps-scoped catalog уже готов для текущего deps/settings snapshot
- **WHEN** handler формирует collect-stage candidates
- **THEN** сервер сначала фильтрует immutable catalog по prefix
- **AND** materialize-ит полные `Candidate` только для surviving subset
- **AND** итоговый candidate set остаётся эквивалентен текущему correctness contract

### Requirement: Background ready-snapshot workers are cooperatively superseded and exact-task promotable (MUST)

Background ready-snapshot workers for `didOpen`/`didChange`/`didSave` MUST behave as controllable
tasks instead of abort-only fire-and-forget jobs.

For obsolete or superseded workers, the system MUST:

- signal cooperative cancellation through shared task state that is observable before and during
  debounce / parse-build execution;
- MUST NOT rely solely on outer async task abort once blocking parse work has already started;
- stop obsolete identical or older-version workers before they continue consuming parser or
  blocking capacity after a newer requested revision has superseded them.

For exact same-version waiters, the system MAY promote an existing worker, but MUST:

- support promotion of an exact same-version worker into `did_save_followup` priority for the
  materialization stage;
- MUST NOT duplicate parse work for identical `(file_id, requested_version, text_hash)`;
- MUST NOT move snapshot-backed `SetFileWithSnapshot` install onto the interactive writer queue
  merely to win the wait.

#### Scenario: Newer didChange supersedes obsolete exact worker before it keeps burning parse capacity

- **GIVEN** a ready-snapshot worker is already running for revision `V`
- **AND** a newer requested revision `V+1` supersedes that file before the older worker finishes
- **WHEN** the system updates worker control state for the file
- **THEN** the older worker observes cooperative cancellation before continuing obsolete parse/build
  work
- **AND** the system does not rely only on outer-task abort to stop already-started blocking parse
  execution

#### Scenario: didSave promotes existing exact worker instead of spawning duplicate parse work

- **GIVEN** `didSave` heavy follow-up needs exact same-version artifacts for revision `V`
- **AND** an exact same-version worker for matching `(file_id, requested_version, text_hash)` is
  already in flight
- **WHEN** the server requests higher priority for that exact worker
- **THEN** the existing worker becomes the promoted producer for that revision
- **AND** the server does not start a second same-version parse worker just because `didSave`
  joined the wait

### Requirement: bsl.getCurrentContext reuses exact same-version snapshot workers before independent parse (MUST)

Backend MUST prefer bounded reuse of an exact same-version ready-snapshot worker before launching
an independent `parser_coordinator` parse for `bsl.getCurrentContext`, when a same-file request
already has a matching in-flight worker for the same text/version.

The backend MUST:

- consume ready exact snapshot state immediately if it is already materialized;
- otherwise wait only a short bounded reuse budget for the exact worker's materialization before
  starting independent parse work;
- preserve latest-generation-wins supersession/cancellation semantics for current-context
  generations;
- fall back to the existing broker/leader parse path if no matching exact task exists, the task no
  longer matches the text/version, or the reuse budget expires.

#### Scenario: currentContext reuses same-file exact worker instead of racing parser_coordinator

- **GIVEN** `didChange` already started an exact same-version ready-snapshot worker for file `F`
  and revision `V`
- **AND** `bsl.getCurrentContext` arrives for the same file text/revision before that worker
  materializes
- **WHEN** the backend decides whether to parse current context independently
- **THEN** it first reuses or briefly awaits the exact worker's materialization
- **AND** only falls back to independent `parser_coordinator` parse if the reuse budget expires or
  the worker stops matching the request
- **AND** newest-generation current-context semantics remain authoritative for the client

### Requirement: Incident bundles distinguish coalesced producer churn from exact timeout (MUST)

Incident-bundle observability MUST expose low-cardinality lifecycle evidence for same-file
ready-snapshot production so operators can distinguish:

- work that was coalesced away before parse;
- work that parsed but was skipped before materialization because a newer target already existed;
- exact same-version producer wait that still timed out and forced `shadow_state` fallback.

This evidence MUST remain bounded and MUST NOT require raw logs to explain whether same-file churn
came from unnecessary worker starts or from a legitimate exact target that still lost to budget.

#### Scenario: Bundle shows coalesced churn instead of masking it as generic superseded work

- **GIVEN** a same-file burst produces several obsolete intermediate revisions before the newest
  exact target materializes
- **WHEN** an operator exports an observability incident bundle
- **THEN** the bundle distinguishes coalesced/retargeted producer outcomes from exact timeout
  outcomes
- **AND** the operator can tell whether `didSave` waited on the right exact producer or whether the
  older revisions were already coalesced away

### Requirement: Ranged `didChange` fallback `stale_parser_base` объясняет, почему reuse-база была недоступна (MUST)

Система MUST дополнительно экспортировать low-cardinality root-cause attribution, если ranged
`didChange` вынужден перейти на full parse с `fallback_reason=stale_parser_base`, чтобы было
понятно, почему дешёвый incremental parser-base reuse не был доступен.

Этот attribution MUST различать как минимум:

- отсутствовал matching ready snapshot для текущего shadow revision;
- latest ready snapshot отставал от shadow revision;
- priming от matching ready snapshot был выполнен, но tree cache всё равно не совпал с shadow
  text;
- иные bounded внутренние причины.

Система MUST экспортировать bounded base-state поля, достаточные для интерпретации miss-класса,
включая текущий shadow revision и latest ready revision, когда они доступны.

Система MUST NOT использовать для этого raw text, free-form debug strings или другую
high-cardinality payload информацию.

#### Scenario: Bundle показывает, что shadow revision ушёл вперёд latest ready base

- **GIVEN** same-file churn продвинул `shadow_state` до revision `V+k`
- **AND** latest ready parse snapshot остаётся на revision `V`
- **WHEN** ranged `didChange` для revision `V+k+1` падает в `stale_parser_base`
- **THEN** observability payload явно показывает bounded miss class, соответствующий lag между
  ready base и shadow revision
- **AND** оператору не нужно открывать raw logs, чтобы понять, что matching ready base для shadow
  revision ещё не существовал

#### Scenario: Bundle показывает mismatch даже после attempted prime

- **GIVEN** для shadow revision существует matching ready snapshot
- **AND** runtime attempted prime parser tree cache from that ready snapshot
- **WHEN** subsequent ranged `didChange` всё равно падает в `stale_parser_base`
- **THEN** observability payload показывает miss class для tree-cache mismatch after prime
- **AND** top-level fallback reason остаётся `stale_parser_base`

### Requirement: Exact ready-snapshot producer экспортирует phase-level latency attribution (MUST)

Система MUST экспортировать bounded phase-level latency attribution для exact ready-snapshot
producer на пути от начала blocking parse до момента, когда ready snapshot уже установлен и
queryable для exact target revision.

Этот attribution MUST различать как минимум:

- `parse_exec`;
- `post_parse_pre_materialization`;
- `ready_install`.

Работа после ready install, включая documentSymbol / outline side-work, MUST экспортироваться
отдельно и MUST NOT искусственно увеличивать exact readiness phase.

#### Scenario: Bundle показывает, что timeout произошёл во время parse phase

- **GIVEN** `didSave` bounded wait ждёт exact still-current ready-snapshot producer
- **AND** budget истекает до materialization
- **WHEN** оператор экспортирует incident bundle
- **THEN** bundle показывает producer phase at timeout
- **AND** если exact worker ещё находился в blocking parse, dominant phase указывает на
  `parse_exec`

#### Scenario: Symbol side-work не маскируется под exact readiness

- **GIVEN** ready snapshot уже установлен для exact target revision
- **AND** после этого ещё выполняется documentSymbol / outline side-work
- **WHEN** observability payload summarises ready-snapshot lifecycle
- **THEN** exact readiness phase заканчивается на ready install
- **AND** symbol/outline side-work показывается как отдельная non-readiness phase

### Requirement: Временный `didSave` exact-wait relief valve является evidence-gated и self-attributing (MUST)

Система MUST ограничивать любое временное дополнительное bounded wait window поверх базового
`didSave` ready-snapshot wait budget только случаями, где runtime может доказать, что:

- ожидание идёт на exact still-current producer для matching
  `(file_id, requested_version, text_hash)`;
- producer не был retargeted/coalesced away;
- наблюдаемый blocker не объясняется runtime queue wait или apply lag;
- exact-path phase attribution показывает late exact readiness, а не generic fallback path.

Если это доказательство отсутствует, система MUST сохранить текущее базовое bounded wait behavior
и MUST перейти к существующему truthful fallback без дополнительного wait window.

Использование временного relief valve MUST оставаться строго bounded, MUST быть явно отражено в
observability / incident bundle export и MUST различать как минимум:

- valve engaged and helped;
- valve skipped because proof was absent;
- valve engaged but still timed out.

#### Scenario: Late exact producer успевает в дополнительное временное окно

- **GIVEN** базовый `didSave` bounded wait исчерпан
- **AND** runtime всё ещё видит тот же exact still-current producer
- **AND** phase attribution показывает late exact readiness без queue/apply-lag признаков
- **WHEN** включён временный relief valve
- **THEN** runtime MAY ждать только в пределах дополнительного bounded relief window
- **AND** если producer materializes внутри этого окна, publish идёт через `ready_artifacts`
- **AND** bundle явно показывает, что relief valve был задействован

#### Scenario: Queue/apply-lag или coalesced-away producer не получают relief window

- **GIVEN** базовый `didSave` bounded wait исчерпан
- **AND** runtime видит apply lag, runtime queue wait или producer уже retargeted/coalesced away
- **WHEN** heavy follow-up выбирает дальнейший путь
- **THEN** runtime MUST NOT включать дополнительное relief wait window
- **AND** использует существующий truthful fallback / attribution path

### Requirement: Ranged `didChange` MUST attempt bounded parser-base recovery before treating `ready_snapshot_lags_shadow_state` as full-parse fate

Система MUST сначала попытаться выполнить bounded parser-base recovery / prime path для текущего
exact document state, если ranged `didChange` попадает в `fallback_reason=stale_parser_base` и
bounded root-cause attribution указывает на `ready_snapshot_lags_shadow_state`.

Этот recovery path MUST:

- оставаться version- and text-bound для exact target revision;
- не использовать raw debug-only state как substitute для matching parser base;
- сохранять существующий truthful fallback, если matching parser base всё ещё не может быть
  доказан.

Система MUST NOT считать `ready_snapshot_lags_shadow_state` достаточной причиной для немедленного
безусловного full parse, если bounded recovery path ещё не был исчерпан.

#### Scenario: Lagging ready snapshot восстанавливает exact parser base без немедленного full parse

- **GIVEN** ranged `didChange` для revision `V+1` видит `shadow_state` на той же revision
- **AND** bounded miss attribution для старого reuse path равен `ready_snapshot_lags_shadow_state`
- **WHEN** runtime запускает bounded parser-base recovery для exact target revision
- **THEN** runtime сначала пытается восстановить/prime matching parser base для этой revision
- **AND** если recovery succeeds, exact ready-snapshot build продолжается без forced full parse из
  этого miss класса

#### Scenario: Recovery failure сохраняет truthful fallback

- **GIVEN** ranged `didChange` попал в miss class `ready_snapshot_lags_shadow_state`
- **AND** bounded parser-base recovery не смог доказать matching parser base
- **WHEN** runtime завершает выбор parse path
- **THEN** система MAY перейти к существующему truthful full fallback
- **AND** observability сохраняет bounded attribution того, что recovery был исчерпан, а exact
  parser base так и не был доказан

### Requirement: Same-file exact ready-snapshot work MUST bound obsolete parse-exec waste

Система MUST иметь bounded cancellation/retarget observation inside the expensive parse/build path,
если same-file churn retargets/coalesces exact ready-snapshot producer на более новую revision,
чтобы obsolete worker мог завершиться во время `parse_exec`, а не только после почти полного
завершения parse cost.

Этот behavior MUST:

- сохранять exact same-version guarantees для surviving worker;
- различать lifecycle/metrics причины как минимум между abort `during_parse_exec` и loss
  `before_materialization`;
- не обвинять `ready_install` или `documentSymbol` phases в obsolete work, которое фактически было
  потрачено внутри parse execution.

#### Scenario: Более новая same-file revision останавливает obsolete worker во время parse execution

- **GIVEN** exact ready-snapshot worker уже выполняет дорогой parse/build path для revision `V`
- **AND** тот же файл получает более новую revision `V+1`, retargeting producer на новый target
- **WHEN** obsolete worker достигает следующего bounded cancellation/retarget checkpoint inside
  `parse_exec`
- **THEN** obsolete worker завершается без materializing revision `V`
- **AND** lifecycle attribution / metrics показывают bounded parse-exec abort, а не поздний
  post-parse loss

#### Scenario: Отсутствие retarget не ломает normal exact materialization

- **GIVEN** exact ready-snapshot worker выполняет parse/build path для current revision
- **AND** новых same-file revisions не приходит до конца build path
- **WHEN** parse/build successfully finishes
- **THEN** ready snapshot materializes как и раньше
- **AND** новые cancellation checkpoints не меняют exact success semantics

### Requirement: Same-version `didSave` follow-up MUST keep exact `parse_exec` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside `parse_exec` whenever `didSave` heavy follow-up waits for an exact still-current
same-version ready-snapshot producer. The runtime MUST keep that exact path focused on
materializing current ready artifacts before optional in-parse work.

This behavior MUST:

- allow non-essential in-parse work to be deferred, skipped, or made cancellable until after exact
  ready artifacts are materialized;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before deferred enrichment

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `parse_exec`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes work required to materialize exact ready artifacts
- **AND** optional in-parse enrichment that is not required for the publishable ready snapshot does
  not block the first exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical producer

- **GIVEN** an exact same-version producer is already on the save-critical path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded in-parse checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `parse_exec` timeouts MUST expose bounded in-parse subphase attribution

The system MUST export a bounded in-parse subphase attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while inside `parse_exec`. This
attribution MUST identify which part of exact `parse_exec` dominated the miss. Operator-facing
observability MUST no longer stop at an opaque phase label when the remaining blocker is entirely
inside exact `parse_exec`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish save-critical parse/build work from deferrable or optional in-parse work;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific in-parse residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `parse_exec`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded in-parse subphase
- **AND** operator-facing output does not collapse the residual back into a single opaque
  `parse_exec` label

#### Scenario: Successful exact publish does not leave stale timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented in-parse attribution is absent or cleared
- **AND** the successful cycle does not report a stale parse timeout residual

### Requirement: Same-version `didSave` follow-up MUST keep exact `core_parse_build` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside exact same-version `core_parse_build` whenever `didSave` heavy follow-up is waiting on a
still-current producer. The runtime MUST keep that exact path focused on the minimum core-build
work required to materialize current ready artifacts before first exact publish.

This behavior MUST:

- allow secondary core-build work that is not required for the first exact ready snapshot to be
  deferred, skipped, or cancelled until after publish;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or newer save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before secondary core-build work

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `core_parse_build`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes the core-build work required for publishable exact ready
  artifacts
- **AND** secondary core-build work that is not required for first publish does not block the first
  exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical core-build producer

- **GIVEN** an exact same-version producer is already on the save-critical core-build path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded core-build checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `core_parse_build` timeouts MUST expose bounded core-build checkpoint attribution

The system MUST export a bounded core-build checkpoint attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while the dominant residual remains
inside `core_parse_build`. Operator-facing observability MUST no longer stop at a monolithic
`core_parse_build` bucket once that bucket becomes the dominant residual after `refactor-27`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish parser/tree work from later exact-ready artifact assembly or other bounded
  core-build slices used by the final implementation;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific core-build residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `core_parse_build`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded core-build checkpoint
- **AND** operator-facing output does not collapse the residual back into a single monolithic
  `core_parse_build` bucket

#### Scenario: Successful exact publish does not leave stale core-build timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented core-build checkpoint attribution is absent or cleared
- **AND** the successful cycle does not report a stale exact core-build timeout residual

### Requirement: Same-version `didSave` follow-up MUST keep exact `ready_snapshot_assembly` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside exact same-version `ready_snapshot_assembly` whenever `didSave` heavy follow-up is waiting
on a still-current producer. The runtime MUST keep that exact path focused on the minimum assembly
work required to materialize current ready artifacts before first exact publish.

This behavior MUST:

- allow secondary assembly work that is not required for the first exact ready snapshot to be
  deferred, skipped, or cancelled until after publish;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or newer save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before secondary assembly work

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `ready_snapshot_assembly`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes the assembly work required for publishable exact ready
  artifacts
- **AND** secondary assembly work that is not required for first publish does not block the first
  exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical assembly producer

- **GIVEN** an exact same-version producer is already on the save-critical assembly path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded assembly checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `ready_snapshot_assembly` timeouts MUST expose bounded assembly checkpoint attribution

The system MUST export a bounded assembly checkpoint attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while the dominant residual remains
inside `ready_snapshot_assembly`. Operator-facing observability MUST no longer stop at a monolithic
`exact_ready_snapshot_assembly` bucket once that bucket becomes the dominant residual after
`refactor-28`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish conversion / packaging slices used by the final implementation;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific assembly residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `ready_snapshot_assembly`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded assembly checkpoint
- **AND** operator-facing output does not collapse the residual back into a single monolithic
  `exact_ready_snapshot_assembly` bucket

#### Scenario: Successful exact publish does not leave stale assembly timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented assembly checkpoint attribution is absent or cleared
- **AND** the successful cycle does not report a stale exact assembly timeout residual

### Requirement: Same-version `didSave` follow-up MUST keep exact `program_conversion` on the save-critical path

The system MUST treat production of publishable exact ready artifacts as the save-critical goal
inside exact same-version `program_conversion` whenever `didSave` heavy follow-up is waiting on a
still-current producer. The runtime MUST keep that exact path focused on the minimum conversion
work required to materialize current ready artifacts before first exact publish.

This behavior MUST:

- allow secondary conversion or packaging work that is not required for the first exact ready
  snapshot to be deferred, skipped, or cancelled until after publish;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve supersession behavior when a newer same-file revision or newer save cycle overtakes the
  current target.

#### Scenario: Save-critical exact producer materializes ready artifacts before secondary conversion work

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `program_conversion`
- **WHEN** runtime promotes that producer onto the save-critical path
- **THEN** the producer prioritizes the conversion work required for publishable exact ready
  artifacts
- **AND** secondary conversion or packaging work that is not required for first publish does not
  block the first exact follow-up publish

#### Scenario: Newer same-file target still supersedes the save-critical conversion producer

- **GIVEN** an exact same-version producer is already on the save-critical conversion path
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded conversion checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `program_conversion` timeouts MUST expose bounded conversion checkpoint attribution

The system MUST export a bounded conversion checkpoint attribution whenever exact same-version
ready-snapshot work still misses the `didSave` follow-up window while the dominant residual remains
inside `program_conversion`. Operator-facing observability MUST no longer stop at a monolithic
`program_conversion` bucket once that bucket becomes the dominant residual after `refactor-29`.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- distinguish conversion / lowering slices from later packaging or ownership-handoff slices used by
  the final implementation;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Exact timeout reports a specific conversion residual

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `program_conversion`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant bounded conversion checkpoint
- **AND** operator-facing output does not collapse the residual back into a single monolithic
  `program_conversion` bucket

#### Scenario: Successful exact publish does not leave stale conversion timeout attribution behind

- **GIVEN** the exact same-version producer finishes in time and `didSave` follow-up publishes
  through `ready_artifacts`
- **WHEN** diagnostics save timeline is finalized
- **THEN** timeout-oriented conversion checkpoint attribution is absent or cleared
- **AND** the successful cycle does not report a stale exact conversion timeout residual

### Requirement: Same-file current-revision apply visibility stays ahead of auxiliary parse churn (MUST)
Система MUST обеспечивать, что после того как `didOpen`, `didChange` или `didSave` уже
зарегистрировал same-file handoff для requested revision `V`, наблюдаемое продвижение
`applied_version >= V` для same-file waiters не задерживается по умолчанию из-за same-file
auxiliary parse work, которая не является самим canonical current-revision handoff.

Этот contract MUST покрывать как минимум:
- interactive current-revision waiters, которые опираются на `wait_for_file_version` или
  semantically equivalent applied-state readiness;
- `didSave` diagnostics heavy follow-up после bounded first publish;
- same-file auxiliary parse/snapshot/context work вроде parse snapshot build, same-version refresh,
  `bsl.getCurrentContext`, `documentSymbol` maintenance, `type_index_precompute` или
  semantically equivalent paths.

Система MAY сохранять bounded writer/runtime architecture, но MUST NOT оставлять newest same-file
waiters в состоянии seconds-scale `apply_lag` / `wait_for_file_version` только потому, что впереди
продолжается auxiliary same-file parse churn.

Если stall все же происходит, operator-facing evidence MUST позволять отличить writer/apply backlog
от downstream semantic/query cost.

#### Scenario: didSave follow-up не ждёт latest applied visibility только из-за same-file auxiliary parse churn
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** `didSave` уже сделал bounded first publish и heavy follow-up ждёт applied visibility для той же revision
- **AND** same-file auxiliary parse work все еще активна для того же файла
- **WHEN** backend продвигает applied-state visibility
- **THEN** heavy follow-up не получает primary delay только из-за этой same-file auxiliary parse work
- **AND** any remaining stall атрибутируется другой bounded причине

#### Scenario: Current-revision completion readiness не остаётся stale из-за same-file auxiliary parse work
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** same-file auxiliary parse work для той же revision все еще выполняется
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness wait не остается stale только потому, что same-file auxiliary parse work ещё не завершилась
- **AND** completion не деградирует в `wait_for_file_version` stall по этой причине как default outcome

### Requirement: Large-module same-version auxiliary parse consumers reuse canonical parse truth (MUST)
Для representative large modules same-version auxiliary parse consumers MUST reuse или coalesce
canonical parse truth, keyed by `(file_id, file_version, text_hash)` или semantically equivalent
identity, вместо того чтобы платить repeated independent cold/full parse по идентичному тексту как
default behavior.

Этот contract MUST покрывать как минимум:
- `build_parse_snapshot_v2` или semantically equivalent version-bound parse snapshot builders;
- same-version save-triggered refresh paths;
- `bsl.getCurrentContext`, когда он читает тот же latest shadow text.

Система MAY выполнить один cold/full parse, если отсутствует previous tree или incremental basis,
но после того как same-version parse truth уже available или in-flight:
- later same-version auxiliary consumers MUST reuse it or coalesce behind it;
- `bsl.getCurrentContext` MUST NOT по умолчанию запускать еще один independent full parse того же текста;
- operator-facing evidence MUST сохранять truthful parse mode/fallback distinction вместо маскировки
  repeated full parse как generic background slowdown.

#### Scenario: Current-context переиспользует in-flight same-version parse truth
- **GIVEN** для большого модуля уже идет same-version parse build для revision `V` и text identity `H`
- **AND** `bsl.getCurrentContext` приходит для того же файла и идентичного latest shadow text
- **WHEN** backend обслуживает оба path
- **THEN** current-context reuse-ит existing same-version parse truth или coalesce-ится behind it
- **AND** не запускает independent full parse identical text как default outcome

#### Scenario: Full-text update не превращает каждый same-version auxiliary consumer в новый cold parse
- **GIVEN** large-module `didChange` для revision `V` изначально упал в full parse path из-за отсутствия incremental basis
- **WHEN** later same-version save refresh или другой auxiliary parse consumer читает тот же text identity
- **THEN** later consumer reuse-ит или coalesce-ит existing same-version parse truth
- **AND** identical text не оплачивается repeated independent full parse по умолчанию

### Requirement: Representative conf_big mixed-load gate separates cold parse regressions from apply backlog (MUST)
Representative real-module acceptance для `conf_big`-class degradation MUST включать same-file
mixed-load profile, который одновременно упражняет:
- `didChange`;
- `didSave`;
- auxiliary parse-only load (`bsl.getCurrentContext` или semantically equivalent path);
- waiter на current-revision visibility (`completion` и/или didSave heavy follow-up).

Этот gate MUST:
- собирать authoritative fields, которые различают parse cold start и apply visibility backlog, как
  минимум parse mode/fallback reason, parse build latency, и applied-state wait fields
  (`apply_changes_queue_wait_ms`, `wait_for_file_version`, `apply_lag`, или semantically equivalent);
- fail-ить, если regression проявляется либо как repeated identical same-version full parse by
  default, либо как seconds-scale applied-version lag при healthy truthful transport seams;
- report-ить parse-cold-start cost и writer/apply backlog как separate failure classes, а не
  схлопывать их в один generic runtime wait bucket.

#### Scenario: Representative gate различает repeated full parse и applied-version backlog
- **GIVEN** representative same-file mixed-load profile на `conf_big`-class fixture
- **WHEN** один sample regression-ит repeated same-version full parse, а другой regression-ит through applied-version lag
- **THEN** gate завершается ошибкой
- **AND** evidence явно различает parse-cold-start failure class и writer/apply backlog failure class

### Requirement: didSave heavy follow-up избегает apply-lag и generic background backlog как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy follow-up того же `save_cycle_sequence` без unbounded зависимости ни от writer/apply lag, ни от generic background runtime backlog как primary gate.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- использовать одну explicit didSave-follow-up lane/policy для writer-owned applied state, same-version fast paths и canonical fallback, а не выводить isolation только из generic diagnostics operation;
- оформлять эту lane как first-class named admission contract (например, `AdmissionLane::DidSaveFollowup` или семантически эквивалентный type-level contract) с canonical additive telemetry/raw-label value `did_save_followup`, отдельный от бинарного `CpuWorkClass` и от `SemanticOperation::Diagnostics`;
- трактовать didSave-follow-up lane identity как first-class admission contract, отдельный от `SemanticOperation::Diagnostics`, и протаскивать его end-to-end через writer/runtime preparation и blocking CPU admission;
- иметь ровно одного owner outer admission arbiter над applied-state / shadow-state / ready-artifacts / fallback branch fan-out; branch-specific code и facade/runtime helpers MUST потреблять выданный opaque lane admission contract вместо собственного branch-local queueing policy;
- не считать change выполненным, если lane identity лишь помечает work, который по-прежнему входит в те же generic `Background` writer FIFO / CPU permit wait paths без отдельного outer admission boundary до scarce resources;
- сохранять бинарную taxonomy `CpuWorkClass` (`Interactive` / `Background`) и реализовывать didSave-follow-up lane как orthogonal admission concern поверх existing non-interactive/background CPU accounting, а не как третье значение work class;
- реализовывать outer admission boundary как explicit latest-wins arbiter/queue перед scarce writer/runtime resources и удерживать один end-to-end follow-up slot от outer admission через writer/runtime preparation, blocking CPU execution и final pre-publish supersession/quota/disposition decision, освобождая scarce slot до outbound publish/output wait, вместо split writer-vs-CPU quotas или raw CPU-only permit semantics;
- применять эту bounded non-interactive follow-up policy к writer-owned applied-state path, если exact same-version applied state уже известен;
- не позволять writer-owned applied-state path обходить новый lane contract через direct snapshot/query helpers вне lane-aware prepare/admission hooks;
- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`, когда это возможно;
- использовать bounded non-interactive follow-up policy, отличную от generic background lane, для same-version fast paths;
- распространять ту же bounded follow-up policy на didSave fallback path через writer/runtime queue, когда fast-path artifacts недоступны;
- требовать explicit lane/supersession admission envelope до входа в scarce writer/runtime resources, а не полагаться только на generic routing из `SemanticOperation::Diagnostics`;
- вырезать didSave-follow-up lane из existing bounded runtime/CPU budget, а не добавлять net-new total process-wide parallelism;
- трактовать operator-visible follow-up lane quota как global process-wide count end-to-end heavy-follow-up slots, охватывающих outer admission boundary, writer/runtime preparation и blocking CPU execution одного follow-up, и MUST NOT раскалывать этот contract на separately configurable writer-vs-CPU quotas или per-file multiplicative capacity;
- при queued contention хранить не более одного queued candidate на файл и ротировать admission fairly между distinct files, чтобы same-file save storm не создавал raw FIFO blocker для другого файла;
- при effective follow-up lane quota `0` не переводить heavy follow-up молча в generic background lane, а отключать новые `didSave + idle_heavy` admissions без влияния на `save_fastlane`;
- при effective follow-up lane quota `0` re-check-ить effective value на admission boundary до захвата scarce writer/runtime resources для queued-but-not-started work и завершать heavy branch canonical non-cancellation outcome `disabled_by_config`, а не silent absence и не generic cancellation surrogate;
- применять runtime quota changes на outer admission boundary для future admissions; already admitted work MAY finish under already acquired slot и MUST NOT быть retroactively reclassified как `disabled_by_config` mid-flight;
- не повышать heavy follow-up до interactive-class semantics;
- не публиковать older-version diagnostics;
- отсекать stale queued follow-up work до захвата scarce follow-up-lane capacity и повторно проверять supersession перед publish, чтобы older same-file save cycle не становился default blocker для newer cycle;
- сохранять latest-wins / supersession semantics для newer save cycles;
- экспортировать dedicated follow-up-lane telemetry additively через bounded first-class canonical `lane` surface, либо semantically equivalent dedicated runtime-lane family, где stable value `did_save_followup` видна отдельно и queue/exec/saturation signals MUST NOT схлопываться в `interactive/background` или legacy `work_class` как единственную видимую форму;
- представлять `disabled_by_config` canonically не только в save trace, но и в terminal diagnostics outcome/disposition reporting через общий outcome/disposition contract как dedicated non-cancellation disposition/outcome, а не как trace-only string;
- оставлять residual blocker explicit в request-centric trace, если contention всё же происходит.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker

#### Scenario: writer-owned applied state still uses the isolated follow-up lane
- **GIVEN** writer уже зарегистрировал exact same-version applied state для saved revision
- **AND** richer didSave follow-up всё ещё требует `snapshot_with_deps` и semantic work
- **WHEN** сервер запускает post-fastlane `idle_heavy` follow-up через applied-state branch
- **THEN** эта ветка использует ту же explicit didSave-follow-up lane policy
- **AND** не обходит lane-aware prepare/admission hooks через direct snapshot/query execution
- **AND** не наследует generic background runtime queue backlog как default primary gate

#### Scenario: one shared outer arbiter owns admission before branch fan-out
- **GIVEN** applied-state, shadow-state, ready-artifacts и fallback follow-up branches all remain reachable
- **WHEN** сервер принимает решение о запуске post-fastlane `idle_heavy` follow-up
- **THEN** outer admission, supersession re-check и slot issuance выполняются ровно один раз до branch fan-out
- **AND** branch-specific code consumes shared lane admission facts instead of implementing an independent queue policy

#### Scenario: unrelated background backlog does not dominate post-fastlane follow-up
- **GIVEN** `didSave` уже дал bounded same-version `save_fastlane` first publish
- **AND** generic background runtime lane насыщена unrelated auxiliary/background work
- **WHEN** сервер запускает richer `idle_heavy` follow-up для того же `save_cycle_sequence`
- **THEN** heavy follow-up не наследует generic background backlog как default admission gate
- **AND** request-centric trace не показывает seconds-scale wait только из-за shared background lane

#### Scenario: fallback path stays isolated from generic background writer/runtime backlog
- **GIVEN** same-version fast-path artifacts для saved revision недоступны
- **AND** didSave follow-up вынужден идти через canonical fallback path с `wait_for_file_version` / `snapshot_with_deps`
- **WHEN** generic background writer/runtime queue насыщена другой background work
- **THEN** didSave follow-up fallback не наследует эту очередь как default primary gate
- **AND** более новый same-file save cycle не застревает за stale follow-up fallback work

#### Scenario: zero lane quota disables new heavy follow-up without silent generic-background fallback
- **GIVEN** effective didSave follow-up lane quota equals `0`
- **WHEN** same-version `save_fastlane` first publish уже завершён и сервер рассматривает новый `idle_heavy` didSave follow-up
- **THEN** сервер не reroute-ит follow-up молча в generic background lane
- **AND** `save_fastlane` semantics для этого save cycle остаются неизменными
- **AND** save trace завершает heavy branch explicit non-cancellation outcome `disabled_by_config`

#### Scenario: queued follow-up re-checks zero quota before scarce-lane admission
- **GIVEN** older same-file `didSave` cycle already queued heavy follow-up before the operator changed the dedicated lane quota
- **AND** effective didSave follow-up lane quota becomes `0` before that queued work acquires scarce lane capacity
- **WHEN** admission is re-evaluated at the lane boundary
- **THEN** queued-but-not-started follow-up does not run on stale pre-disable assumptions
- **AND** the heavy branch finishes canonical non-cancellation outcome `disabled_by_config`

#### Scenario: already admitted follow-up is not retroactively disabled by later quota change
- **GIVEN** same-file `didSave` heavy follow-up already crossed the dedicated outer admission boundary and owns an end-to-end follow-up slot
- **AND** the operator lowers the lane quota after that admission, including the case `quota=0`
- **WHEN** the already admitted heavy branch continues toward terminal disposition
- **THEN** it is not reclassified mid-flight as `disabled_by_config`
- **AND** the updated quota governs only subsequent outer-admission decisions

#### Scenario: stale queued follow-up yields to a newer save cycle before monopolizing the isolated lane
- **GIVEN** older same-file `didSave` cycle уже поставил heavy follow-up в dedicated lane
- **AND** более новый same-file `didSave` cycle supersedes older cycle before older follow-up acquires or meaningfully consumes lane capacity
- **WHEN** scheduler/admission policy re-evaluates queued older follow-up
- **THEN** obsolete work is shed before becoming the default blocker for the newer cycle
- **AND** newer cycle keeps latest-wins semantics for both first publish and heavy follow-up

#### Scenario: global single-slot quota does not let one file build a raw FIFO wall for another file
- **GIVEN** effective didSave follow-up lane quota equals `1`
- **AND** file A produces repeated same-file `didSave` cycles while file B also has queued heavy follow-up
- **WHEN** the outer arbiter chooses the next queued admission
- **THEN** the queue retains only the latest queued candidate per file
- **AND** file B is not stranded behind an unbounded FIFO of superseded file-A entries
- **AND** total admitted heavy follow-up work still does not exceed the global quota

#### Scenario: one admitted follow-up owns one scarce slot until the pre-publish disposition decision
- **GIVEN** `didSave` heavy follow-up crossed the dedicated outer admission boundary
- **WHEN** that follow-up performs writer/runtime preparation, blocking semantic execution и затем проходит final pre-publish supersession/quota/disposition decision
- **THEN** the same bounded follow-up slot remains owned through that decision
- **AND** outbound publish/output wait, if any, does not continue monopolizing the scarce didSave-follow-up slot
- **AND** the implementation does not reinterpret the same work through separate writer-vs-CPU lane quotas

#### Scenario: dedicated follow-up-lane telemetry stays separately attributable
- **GIVEN** didSave heavy follow-up uses the isolated lane under contention or execution
- **WHEN** runtime metrics and request-centric traces are exported
- **THEN** queue/exec/saturation facts for that lane stay separately attributable
- **AND** operators do not need to infer the lane only from generic `interactive/background` buckets
- **AND** any compatibility projection into legacy buckets or binary `CpuWorkClass` / `work_class` views does not replace the dedicated lane representation
- **AND** canonical additive telemetry exposes stable lane identity `did_save_followup` through a bounded lane surface or semantically equivalent dedicated runtime-lane family

#### Scenario: nominal background retagging without outer gate is rejected
- **GIVEN** implementation adds a didSave-follow-up marker but still routes queued work through the same generic `Background` scarce admission points
- **WHEN** older queued follow-up or `quota=0` must be re-evaluated before scarce capacity is consumed
- **THEN** such implementation does not satisfy the requirement
- **AND** the dedicated lane contract is considered unmet until an explicit outer admission boundary exists

#### Scenario: residual contention stays explicit after follow-up isolation
- **GIVEN** heavy didSave follow-up всё же сталкивается с residual contention
- **WHEN** diagnostics save trace экспортирует terminal или in-flight состояние
- **THEN** trace сохраняет explicit request-centric blocker facts
- **AND** не подменяет remaining delay на guessed generic `pending`

### Requirement: Transport runtime progression stays task-isolated from scheduler/service work (MUST)
The system MUST keep transport runtime progression task-isolated from scheduler/service work.

Transport runtime loops, отвечающие за adapter read/decode/classify, single-owner scheduling и
output/handoff progression, MUST выполняться на independently progressing async tasks или на
эквивалентной starvation-safe boundary. Long-running pre-await work, readiness wait или barrier
handling в одном loop MUST NOT по конструкции останавливать остальные loops.

Этот contract MUST гарантировать как минимум:

- поздний adapter read/decode/classify может продолжаться, даже если scheduler уже занят stalled
  request;
- ready output/flush progression может продолжаться, даже если input/scheduler остаются заняты
  другим request path;
- pre-await work inside document-sync or barrier-related futures MAY существовать, но MUST NOT
  монополизировать тот же async task, что обслуживает transport reader или output writer;
- task-isolation MUST сохранять existing single-owner `poll_ready()/call()` semantics, а не
  заменять её конкурентными вызовами в несколько owners.

#### Scenario: Stalled scheduler branch не мешает позднему cancel быть классифицированным

- **GIVEN** scheduler уже держит stalled request branch до dispatch
- **AND** затем transport получает новый `$/cancelRequest`
- **WHEN** transport runtime продолжает работу
- **THEN** reader продолжает read/decode/classify нового control request
- **AND** cancel не застревает только потому, что stalled scheduler branch живёт на том же async
  task

#### Scenario: Ready response flush не стоит за unrelated scheduler stall

- **GIVEN** один request уже подготовил user-facing response и готов к output/flush progression
- **AND** другой request всё ещё держит scheduler path в stalled state
- **WHEN** transport runtime продолжает работу
- **THEN** ready response flush progresses independently
- **AND** output path не ждёт завершения unrelated scheduler stall только из-за same-task topology

### Requirement: Same-version `didSave` heavy follow-up MUST wake on the first matching diagnostics artifact within the bounded wait window

The system MUST treat canonical live `ready_artifacts` materialization and detached
diagnostics-ready artifact publication as two distinct bounded wake sources for the same
still-current `didSave` target whenever heavy follow-up is waiting on same-version readiness for
that target.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent same-save identity;
- prefer canonical `ready_artifacts` immediately when they are already materialized;
- during the bounded wait, race canonical `ready_artifacts` materialization against matching
  detached diagnostics-ready artifact publication for that same target;
- allow canonical `ready_artifacts` to win if live exact readiness materializes first;
- allow detached diagnostics-ready artifacts to win only while canonical live exact readiness is
  still pending for the same target;
- use a cancellation-safe or semantically equivalent restart-safe wake surface so repeated
  wait-loop restarts do not lose detached publication events;
- preserve latest-wins supersession, diagnostics-generation matching, version matching,
  `save_cycle_sequence` matching, cancellation, and truthful miss outcomes when the target is no
  longer current;
- preserve fail-closed semantics for `hover`, `definition`, `signatureHelp`, completion exact
  upgrade, and semantically equivalent interactive exact consumers until canonical live exact
  readiness completes;
- export operator-facing evidence that names which wake source won (`ready_artifacts`,
  `detached_ready_artifacts`, or a truthful miss outcome) and how long the bounded wait lasted;
- MUST NOT satisfy this requirement by widening the bounded wait budget or by treating detached
  diagnostics-ready state as canonical live exact readiness.

#### Scenario: Detached diagnostics-ready publication wins the bounded wait before canonical timeout

- **GIVEN** `didSave` heavy follow-up is waiting on a still-current same-version target
- **AND** canonical live exact `ready_artifacts` are not yet materialized for that target
- **AND** a matching detached diagnostics-ready artifact is published during the bounded wait
- **WHEN** the waiter resolves the first matching wake source
- **THEN** the heavy follow-up completes through `detached_ready_artifacts`
- **AND** it does not burn the rest of the bounded wait budget merely because canonical
  `ready_install` is still pending
- **AND** exported evidence names `detached_ready_artifacts` as the wake winner

#### Scenario: Canonical ready artifacts still win if they materialize first

- **GIVEN** `didSave` heavy follow-up is waiting on a still-current same-version target
- **AND** both canonical ready-artifact materialization and detached publication are possible for
  that target
- **WHEN** canonical live exact `ready_artifacts` materialize before any matching detached wake
- **THEN** the heavy follow-up completes through `ready_artifacts`
- **AND** detached diagnostics-ready publication, if it appears later, does not rewrite the winner
  for that wait

#### Scenario: Stale detached publication does not wake a newer still-current target

- **GIVEN** a newer same-file revision, diagnostics generation, or `save_cycle_sequence` has
  already overtaken an older waiting target
- **AND** a detached diagnostics-ready artifact is published for the older target
- **WHEN** the newer waiter evaluates the detached wake source
- **THEN** it ignores the stale detached publication
- **AND** terminal behavior remains truthful through supersession, mismatch, cancellation, or
  another bounded miss outcome

### Requirement: Same-version `didSave` follow-up MUST use detached diagnostics-ready artifacts without weakening live exact gates

The system MUST allow same-version `didSave` heavy follow-up to complete from a detached
diagnostics-ready artifact when bounded exact work has already produced the diagnostics-ready
payload for the still-current target but canonical live exact readiness for that same target is
still blocked on `ready_install`, type-index publication, or semantically equivalent live install
work.

This behavior MUST:

- remain keyed to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)` target,
  or a semantically equivalent same-save identity;
- publish the detached artifact outside canonical live current-revision exact readiness and outside
  APIs that interactive exact consumers treat as proof of exact readiness;
- keep the detached artifact bounded to diagnostics-save follow-up, request-centric incident
  bundle export, or semantically equivalent diagnostics-only consumers;
- allow `didSave` follow-up to prefer the detached artifact over terminal `shadow_state` fallback
  when the target remains still-current and the detached artifact is already materialized;
- preserve exact same-version semantics, latest-wins supersession, cancellation, and truthful
  fallback when a newer same-file revision or newer save cycle overtakes the target or detached
  proof is exhausted;
- preserve fail-closed semantics for `hover`, `definition`, `signatureHelp`, `type-at-position`,
  completion exact upgrade, and semantically equivalent interactive exact consumers until
  canonical live exact readiness completes;
- preserve operator-facing evidence that distinguishes detached diagnostics-ready consumption from
  canonical live `ready_artifacts`, degraded `shadow_state`, and superseded outcomes;
- MUST NOT satisfy this requirement by early-publishing snapshot-backed live exact state,
  `SetFileWithSnapshot`, or semantically equivalent partial install that makes diagnostics-ready
  state look like canonical current-revision exact readiness.

#### Scenario: `didSave` follow-up uses detached diagnostics-ready artifacts while live install is still pending

- **GIVEN** a same-version exact producer already built the bounded diagnostics-ready payload for a
  `didSave` target
- **AND** canonical live exact readiness for that target is still blocked on `ready_install`,
  type-index publication, or semantically equivalent live install work
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** `didSave` heavy follow-up resolves the still-current target
- **THEN** the follow-up completes through the detached diagnostics-ready artifact
- **AND** it does not keep live exact install as the primary gate for that diagnostics-only path
- **AND** exported evidence identifies detached diagnostics-ready consumption rather than terminal
  `shadow_state`

#### Scenario: Interactive exact consumers remain fail-closed until canonical live readiness exists

- **GIVEN** a detached diagnostics-ready artifact already exists for revision `V`
- **AND** canonical live exact readiness for revision `V` is still unavailable
- **WHEN** the IDE requests `hover`, `definition`, `signatureHelp`, `type-at-position`, or
  semantically equivalent exact behavior for revision `V`
- **THEN** the request does not treat the detached artifact as canonical exact truth
- **AND** the existing live exact-readiness / fail-closed policy remains in force

#### Scenario: Superseded same-file target does not leak detached diagnostics artifacts

- **GIVEN** a detached diagnostics-ready artifact exists for an older same-file revision or older
  `save_cycle_sequence`
- **WHEN** a newer same-file revision or newer save cycle overtakes that target
- **THEN** the older detached artifact is not consumed as the answer for the newer target
- **AND** terminal disposition remains truthful through supersession, cancellation, or another
  bounded fallback outcome

### Requirement: Same-version `didSave` follow-up MUST keep exact `parser_base_recovery` on the save-critical path

The system MUST treat matching parser-base proof or recovery as save-critical work only to the
extent required to resume exact ready-snapshot materialization for the still-current same-version
target whenever `didSave` heavy follow-up is waiting on that target and the dominant exact blocker
remains `parser_base_recovery`.

This behavior MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target;
- when `didSave` follow-up observes a matching still-current in-flight same-version producer,
  promote and wait on that producer rather than bypassing it through a parallel didSave-only
  semantic branch;
- keep `parser_base_recovery` focused on bounded work required to prove or install a matching
  parser base for that exact target before later tree-build or exact-artifact work proceeds;
- preserve the existing bounded wait and relief-valve budgets as the primary latency envelope and
  MUST NOT rely on widening them as the primary remedy;
- treat exhausted recovery proof as bounded failure to match/install the parser base or to advance
  the still-current producer beyond `parser_base_recovery` into a later exact checkpoint within the
  existing envelope, and MUST NOT treat mere continued elapsed time inside the unchanged checkpoint
  as sufficient proof;
- preserve exact same-version semantics for any produced ready snapshot;
- preserve latest-wins supersession and cancellation when a newer same-file revision or newer save
  cycle overtakes the target;
- preserve truthful fallback to degraded paths only when bounded recovery proof is exhausted or the
  target is superseded.

#### Scenario: Still-current same-version producer leaves `parser_base_recovery` in bounded time

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the dominant exact blocker would otherwise remain `parser_base_recovery`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** runtime executes save-critical parser-base recovery for the exact target
- **THEN** the producer prioritizes only the bounded recovery work required to prove or install a
  matching parser base for that target
- **AND** `didSave` follow-up keeps waiting on that promoted producer rather than switching to a
  parallel didSave-only semantic branch
- **AND** the path reaches later exact work or materializes ready artifacts without falling back to
  `shadow_state` solely because `parser_base_recovery` monopolized the same-version exact path

#### Scenario: Exhausted recovery proof preserves truthful fallback

- **GIVEN** `didSave` heavy follow-up is waiting on an exact same-version producer
- **AND** bounded save-critical parser-base recovery cannot prove or install a matching parser base,
  or cannot move the still-current producer beyond `parser_base_recovery` within the existing
  envelope
- **WHEN** runtime exhausts that recovery proof
- **THEN** the system MAY fall back truthfully to the existing degraded path
- **AND** observability preserves that `parser_base_recovery` was the exhausted blocker rather than
  hiding the incident under a generic parse delay
- **AND** the fallback is not justified solely by additional wall time spent in the same unchanged
  `parser_base_recovery` checkpoint

### Requirement: Diagnostics-only semantic simplification MUST NOT regress later LSP exact consumers on the same current revision

The system MUST preserve canonical current-revision exact semantics for LSP exact consumers after a
diagnostics-only semantic path has already executed for that same revision.

At minimum this requirement applies to:

- `textDocument/hover`;
- `textDocument/definition`;
- and any other LSP semantic query that shares their exact-only runtime path in the final
  implementation.

This behavior MUST:

- keep diagnostics-only artifacts non-substitutable for exact LSP semantic queries;
- keep later hover/definition requests able to reach the canonical exact artifact for the same
  current revision when that artifact is already ready or becomes ready through the existing
  bounded exact-readiness policy;
- preserve fail-closed empty/unavailable behavior when the exact current-revision artifact is
  genuinely unavailable within bounded policy;
- preserve the current serve-only / fail-closed contract for LSP exact consumers and MUST NOT be
  satisfied by silently re-enabling hidden on-demand exact materialization on the LSP request
  path;
- NOT be satisfied by silently widening diagnostics-only materialization until it effectively
  becomes a second exact contract;
- preserve bounded fail-closed reason-code observability for genuine exact misses.

#### Scenario: Same-revision hover and definition still recover canonical exact semantics after diagnostics-only path

- **GIVEN** a diagnostics-only semantic path has already run for the current document revision
- **AND** a later LSP hover or goto-definition request needs canonical exact semantics for that
  same revision
- **AND** the exact artifact for that revision is already ready or becomes ready through the
  existing bounded exact-readiness policy
- **WHEN** the runtime serves the LSP request
- **THEN** it serves the request from the canonical exact artifact path for that revision
- **AND** it does not treat the diagnostics-only artifact as a successful exact cache hit
- **AND** hover/definition return the expected exact result

#### Scenario: Genuine exact miss remains fail-closed after diagnostics-only path

- **GIVEN** a diagnostics-only semantic path has already run for the current document revision
- **AND** the exact current-revision artifact is still genuinely unavailable within bounded policy
- **WHEN** LSP hover or goto-definition is requested
- **THEN** the response remains empty or unavailable according to the API contract
- **AND** the runtime does not rescue the request with stale, search-only, or diagnostics-only
  semantic substitutes

### Requirement: Same-version `didSave` follow-up MUST bound exact `parse_exec` residence before the first subphase callback

The system MUST bound the opaque pre-subphase `parse_exec` residence of a still-current
same-version exact ready-snapshot producer whenever `didSave` heavy follow-up is waiting on that
producer.

This behavior MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash, save_cycle_sequence)`
  target, or a semantically equivalent per-save-cycle identity;
- treat the region currently observable as `before_first_parse_exec_subphase` as part of the
  save-critical exact path rather than as an unbounded invisible entry span;
- either materially reduce that representative blocked interval or expose truthful bounded internal
  progress for the same target before the steady-state follow-up latency is dominated by that
  region;
- preserve the current bounded wait and relief-valve budgets as the primary latency envelope;
- NOT be satisfied solely by widening those budgets;
- NOT be satisfied solely by relabelling the same opaque interval under another observability
  bucket without reducing or truthfully subdividing it for the same target;
- preserve exact same-version semantics for any produced ready artifacts;
- preserve latest-wins supersession, retarget, and cancellation behavior when a newer same-file
  revision or newer save cycle overtakes the target;
- preserve operator-facing low-cardinality evidence distinguishing still-current continuation,
  exhausted continuation proof, supersession, and cancellation.

#### Scenario: Still-current same-version producer reaches bounded progress before opaque pre-subphase `parse_exec` dominates

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the representative timeout leaf would otherwise be `before_first_parse_exec_subphase`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** runtime executes the representative save-follow-up policy
- **THEN** the producer reaches a bounded first in-parse progress point or materializes exact ready
  artifacts in time for the representative path to avoid spending its steady-state latency inside
  one opaque pre-subphase `parse_exec` span
- **AND** the heavy follow-up remains on `ready_artifacts`

#### Scenario: Newer target still supersedes the pre-subphase producer truthfully

- **GIVEN** an exact same-version producer is still inside bounded pre-subphase `parse_exec`
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded checkpoint
- **THEN** the producer MAY terminate, retarget, or fall back truthfully instead of publishing
  stale output
- **AND** the system does not keep an obsolete target alive merely to avoid reporting pre-subphase
  attribution

### Requirement: Exact same-version `program_lowering` MUST materialize safe reuse without a second deep-clone of unchanged regions

The system MUST, when applying a conservative exact same-version lowering-reuse plan for the
current ready-snapshot target, materialize reused top-level lowering units and reused callable-body
statement windows by ownership transfer or an equivalently bounded no-extra-clone path rather than
deep-cloning the unchanged subtree a second time before final `Program` assembly.

This behavior MUST:

- preserve the fail-closed invalidation boundaries introduced by
  `refactor-33-exact-program-lowering-changed-range-reuse`;
- preserve exact same-version semantics, latest-wins supersession, and truthful
  cancellation/retarget behavior for save-follow-up exact assembly;
- remove the second full-subtree deep-clone during final `Program` assembly for reused regions;
- allow at most one bounded rebase/update pass needed to align moved reused nodes to the current
  revision, rather than silently expanding this change into a broader structural-sharing rewrite.

#### Scenario: Local same-file edit reuses unchanged lowered regions without a second deep clone

- **GIVEN** the previous exact ready snapshot already proved many top-level lowering units and
  callable-body statement windows unchanged
- **AND** the current same-file target still qualifies for conservative reuse under the existing
  invalidation rules
- **WHEN** exact `program_lowering` materializes the final `Program`
- **THEN** the runtime moves those unchanged regions into the final assembly through the consumed
  reuse plan
- **AND** it does not deep-clone the full reused subtree a second time solely to rebuild the final
  `Program`

#### Scenario: Ambiguous invalidation still rebuilds instead of reusing

- **GIVEN** a same-file edit touches or may affect a lowering boundary whose reuse soundness is not
  proven
- **WHEN** exact `program_lowering` derives or applies the reuse plan
- **THEN** the affected region is rebuilt fail-closed
- **AND** the runtime does not use ownership-based materialization to bypass rebuild eligibility

### Requirement: Exact reuse observability MUST remain truthful after ownership-based plan consumption

The system MUST preserve truthful reuse-versus-rebuild attribution for one traced exact same-file
save-follow-up target even when the lowering-reuse plan is consumed by ownership during final
`program_lowering` assembly.

This evidence MUST include at least:

- the reuse-plan outcome for the traced exact target;
- bounded reused-versus-rebuilt lowering workload counts;
- the residual exact `program_lowering` latency for that same traced target.

#### Scenario: Representative follow-up still explains reduced exact lowering work truthfully

- **GIVEN** ownership-based reuse materialization is enabled for a representative same-file
  save-follow-up target
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the report still exposes both the exact `program_lowering` residual and the
  reused-versus-rebuilt lowering breakdown for that traced target
- **AND** operators can distinguish "less work was materialized" from "the same work was merely
  relabeled"

### Requirement: Representative same-file save-follow-up MUST bound diagnostics-only semantic query residual once the exact path is stable

The system MUST reduce diagnostics-only semantic query latency on the representative same-file
`didSave` heavy follow-up family once that family already remains on current exact
`ready_artifacts`, without regressing exactness truthfulness.

This behavior MUST:

- preserve the current exact `ready_artifacts` path for supported representative same-file
  save-follow-up targets;
- preserve diagnostics-only semantic materialization for supported cases, or preserve truthful full
  fallback when parity cannot be proven;
- NOT be satisfied solely by widening upstream wait budgets or by silently shifting supported
  diagnostics-only work onto the full semantic-facts path;
- preserve operator-facing evidence that distinguishes diagnostics-only current-exact work from
  full fallback and shows where the dominant semantic residual moved.

#### Scenario: Representative family stays exact while diagnostics-only semantic query residual drops

- **GIVEN** a representative same-file save-follow-up family already publishes through current exact
  `ready_artifacts`
- **AND** diagnostics-only semantic query is the dominant remaining residual on that family
- **WHEN** the runtime executes semantic diagnostics for that representative family
- **THEN** refreshed representative evidence shows lower diagnostics-only semantic query latency
  than the checked-in `refactor-39` baseline
- **AND** the family still remains on `ready_artifacts`
- **AND** the traced semantic path remains diagnostics-only unless a truthful full fallback is
  required

#### Scenario: Unsupported optimization does not fake a latency win through silent fallback

- **GIVEN** an attempted diagnostics-only optimization cannot preserve semantic parity for the
  current exact target
- **WHEN** the runtime executes semantic diagnostics for that target
- **THEN** it preserves truthful diagnostics-only versus full-fallback attribution
- **AND** it does not claim success by silently downgrading supported work to full fallback or by
  publishing stale semantic results

### Requirement: Diagnostics-only semantic evidence MUST export path-specific leaf attribution

When semantic diagnostics use the diagnostics-only materialization path, the system MUST export
path-specific leaf attribution for the diagnostics-only semantic-facts builder rather than only an
aggregate diagnostics-only IR total.

At minimum this evidence MUST distinguish:

- AST->IR conversion time;
- diagnostics-only semantic-facts build subphases that actually ran for the traced target;
- diagnostics collection time after diagnostics-only materialization;
- the traced diagnostics semantic materialization path for that target;
- the absence of full-semantic-facts-only subphases that did not run on that path.

The diagnostics-only leaf surface MUST use a dedicated diagnostics-only field family or equivalent
dedicated namespace.

Reusing the existing `semantic_diagnostics_ir_semantic_facts_*` full-path field family for
diagnostics-only work MUST NOT satisfy this requirement, even if the old fields are accompanied by
best-effort comments or indirect cumulative metrics.

The exported diagnostics-only leaf attribution MUST be sourced from the diagnostics-only builder
profile returned by `analysis-v2` rather than heuristically reconstructed only downstream.

#### Scenario: Representative report explains the diagnostics-only residual truthfully

- **GIVEN** a representative same-file save-follow-up uses diagnostics-only semantic
  materialization
- **WHEN** the runtime exports the traced diagnostics report
- **THEN** the report includes diagnostics-only leaf attribution for that traced target
- **AND** the report includes the traced diagnostics semantic `materialization_path`
- **AND** skipped full-semantic-facts-only subphases stay absent or zero
- **AND** operators can see whether the remaining residual is in AST->IR, diagnostics-only facts
  build, or diagnostics collection

#### Scenario: Reusing the old full-path leaf family without traced path identity is rejected

- **GIVEN** an implementation exports diagnostics-only timings only through the old
  `semantic_diagnostics_ir_semantic_facts_*` field family or omits the traced
  `materialization_path`
- **WHEN** the representative diagnostics report is reviewed
- **THEN** the requirement is not satisfied
- **AND** the diagnostics-only leaf surface is still considered ambiguous

### Requirement: Same-version `didSave` follow-up MUST bound terminal `shadow_state` fallback while a still-current exact producer remains in `parse_exec`

The system MUST prefer a bounded still-current exact path when `didSave` heavy follow-up is
waiting on an exact same-version producer that is still current and already inside bounded
`parse_exec`.

On the representative save-follow-up family, terminal `shadow_state` fallback MUST remain a
truthful exception rather than the steady-state outcome for that state.

This behavior MUST:

- remain bound to the exact `(file_id, requested_version, text_hash, save_cycle_sequence)`
  target, or a semantically equivalent per-save-cycle identity;
- preserve the existing bounded wait and relief-valve budgets as the primary latency envelope;
- NOT be satisfied solely by widening those budgets instead of improving still-current producer
  continuity, proof, or promotion behavior;
- avoid repeatedly terminating the heavy follow-up on
  `wait_probe=timeout -> relief_valve=engaged_timed_out -> shadow_state` solely because the
  initial bounded wait elapsed while the same producer remained the newest valid target;
- preserve exact same-version semantics for any produced ready artifacts;
- preserve truthful supersession, cancellation, and fallback when a newer same-file revision or
  newer save cycle overtakes the current target, or when the runtime can no longer prove that the
  in-flight producer remains the bounded best candidate;
- preserve operator-facing low-cardinality evidence that distinguishes:
  - a still-current exact continuation path that remained eligible after the initial timeout;
  - a terminal `shadow_state` fallback because still-current continuation proof was exhausted;
  - truthful supersession, cancellation, or other terminal non-continuation outcomes.

#### Scenario: Still-current same-version `parse_exec` producer wins the heavy follow-up path

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish
- **AND** the heavy follow-up is waiting on a still-current exact same-version producer that is
  already inside bounded `parse_exec`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the runtime executes the representative save-follow-up policy
- **THEN** the heavy follow-up publishes through `ready_artifacts`
- **AND** `shadow_state` is not the terminal branch solely because the initial bounded wait elapsed

#### Scenario: Truthful fallback remains when the current exact target is no longer provable

- **GIVEN** the heavy follow-up exhausted its initial bounded wait on an exact same-version
  producer
- **AND** either a newer same-file revision or newer save cycle overtakes that target, or the
  runtime can no longer prove that the in-flight producer remains the bounded best candidate
- **WHEN** the runtime finalizes the follow-up path
- **THEN** it MAY terminate truthfully through `shadow_state` or `superseded_generation`
- **AND** the exported evidence preserves whether still-current continuation was attempted
- **AND** the exported evidence preserves why the still-current exact path was not chosen

### Requirement: Semantic diagnostics MUST support diagnostics-only type-hint materialization for the current exact target

The system MUST support a diagnostics-only semantic path that materializes only the type-hint
artifact required by semantic diagnostics for the current exact target instead of always
materializing full `SemanticFacts`.

At minimum this diagnostics-only artifact MUST support:

- `assignment_value_type_by_span`;
- `call_receiver_type_by_span`;
- `call_arg_types_by_span`;
- `member_access_object_type_by_span`.

This behavior MUST:

- preserve semantic diagnostics parity with the full semantic path for supported cases;
- fall back to the full semantic path fail-closed when parity cannot be proven for a case;
- avoid performing diagnostics-irrelevant full semantic-facts work on supported same-file
  save-follow-up targets.

#### Scenario: Representative semantic diagnostics use diagnostics-only hints instead of full semantic facts

- **GIVEN** a same-file save-follow-up requests semantic diagnostics for a current exact target
- **AND** that target falls within the supported diagnostics-only contract
- **WHEN** the runtime materializes semantic inputs for diagnostics
- **THEN** it builds diagnostics-only type hints instead of full `SemanticFacts`
- **AND** the resulting semantic diagnostics remain equivalent to the full semantic path

#### Scenario: Unsupported diagnostics case falls back to the full semantic path

- **GIVEN** semantic diagnostics encounter a case whose parity is not proven under the
  diagnostics-only contract
- **WHEN** the runtime prepares semantic inputs for diagnostics
- **THEN** it falls back to full `SemanticFacts`
- **AND** it does not silently publish reduced diagnostics from an unsupported narrowed path

### Requirement: Diagnostics-only semantic artifacts MUST remain isolated from the full exact semantic artifact cache

The system MUST NOT store diagnostics-only semantic artifacts under the current full exact semantic
cache identity for the same `(file, version, deps, settings)` target.

Diagnostics-only artifacts MUST be ephemeral or stored under a separate diagnostics cache namespace
so later interactive exact consumers cannot mistake them for full `SemanticFacts`.
This isolation requirement also applies to any cached `SemanticProgram`, completion-head artifact,
or equivalent exact IR-derived artifact that interactive exact consumers reuse.
The diagnostics-only path MUST NOT publish a trimmed semantic artifact into the current exact
interactive slot for that target.

#### Scenario: Diagnostics-only query does not poison later interactive exact requests

- **GIVEN** a diagnostics-only semantic query already ran for the current exact target
- **WHEN** a later interactive exact request such as hover, completion, definition,
  `signatureHelp`, or type-at-position needs full semantic facts
- **THEN** the runtime does not treat the diagnostics-only artifact as a cache hit for the full
  semantic contract
- **AND** the interactive request still reads or builds full `SemanticFacts`

### Requirement: Representative diagnostics evidence MUST distinguish diagnostics-only hints from full semantic-facts fallback

The system MUST export low-cardinality attribution showing whether representative semantic
diagnostics used diagnostics-only hint materialization or fell back to full `SemanticFacts`.

This evidence MUST include at least:

- the diagnostics semantic path identity for the traced target;
- the bounded latency for diagnostics-hint materialization or full semantic-facts fallback;
- the remaining diagnostics collection/query latency for that same traced target.

#### Scenario: Representative report explains the diagnostics semantic path truthfully

- **GIVEN** a representative same-file save-follow-up exports semantic diagnostics evidence
- **WHEN** the diagnostics path finishes or exports a checked-in report
- **THEN** the report distinguishes diagnostics-only hint materialization from full semantic-facts
  fallback for that traced target
- **AND** operators can attribute the residual to the correct semantic path instead of inferring it
  indirectly

### Requirement: Exact same-version `program_lowering` MUST avoid whole-callable body rebuild for bounded local edits when safe

The system MUST, for an exact same-version ready-snapshot target whose changed ranges stay inside
one callable body, derive a conservative callable-body partial-rebuild plan when body-local
invalidation boundaries can be proven safely.

When that plan proves that only a bounded local region inside the callable body is invalidated, the
runtime MUST rebuild only the invalidated statement window and any semantically dependent enclosing
control-flow region, rather than recursively dispatching every statement in the callable body.

This behavior MUST:

- stay bound to the exact `(file_id, requested_version, text_hash)` target;
- preserve the fail-closed invalidation discipline established by
  `refactor-33-exact-program-lowering-changed-range-reuse`;
- preserve exact same-version semantics, latest-wins supersession, and truthful
  cancellation/retarget behavior;
- rebuild the whole callable body instead of guessing when body-local soundness is not proven.

#### Scenario: Bounded local edit inside one large callable body rebuilds only the invalidated body window

- **GIVEN** the previous same-file revision already has an exact ready snapshot
- **AND** the new revision changes only a bounded local region inside one large callable body
- **AND** the runtime can prove safe body-local invalidation boundaries for that edit
- **WHEN** exact ready-snapshot assembly lowers the still-current target
- **THEN** the runtime rebuilds only the invalidated body-local region and any semantically
  dependent enclosing control-flow region
- **AND** it does not recursively dispatch the whole callable body solely because that one local
  edit occurred

#### Scenario: Ambiguous body-local invalidation falls back to whole-callable rebuild

- **GIVEN** a same-file edit inside one callable body touches or may affect a body-local boundary
  whose rebuild soundness is not proven
- **WHEN** exact ready-snapshot assembly derives or applies the callable-body partial-rebuild plan
- **THEN** the affected callable body is rebuilt fail-closed
- **AND** the runtime does not guess a narrower partial-rebuild boundary

### Requirement: Representative exact lowering observability MUST expose rebuilt callable-body work

The system MUST export operator-facing evidence showing how much direct rebuilt callable-body work
remains for one traced exact same-file save-follow-up target on representative large-module churn.

This evidence MUST include at least:

- the exact `program_lowering` residual for the traced target;
- whether the rebuilt callable used bounded body-local rebuild or whole-callable fallback;
- direct rebuilt callable-body dispatch time and call count for that traced target.

#### Scenario: Representative follow-up explains parser residual using rebuilt callable-body metrics

- **GIVEN** a representative large-module same-file save follow-up exercises the exact path after
  the callable-body partial-rebuild change
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the evidence shows the exact `program_lowering` residual and the direct rebuilt
  callable-body metrics for that traced target
- **AND** operators can distinguish "less callable-body work was rebuilt" from "the same parser
  hotspot was only relabeled"

#### Scenario: Whole-callable fallback remains truthful in representative evidence

- **GIVEN** the traced exact target falls back to whole-callable rebuild because body-local
  boundaries are ambiguous
- **WHEN** observability exports the representative follow-up result
- **THEN** the report truthfully indicates that bounded callable-body partial rebuild did not
  qualify
- **AND** the direct rebuilt callable-body metrics remain coherent for that fallback path

### Requirement: Canonical local-function-summary inference MUST short-circuit singleton non-recursive SCCs

The system MUST, when canonical semantic-facts materialization derives local routine summaries for
the current exact target revision, detect singleton SCCs that have no self-edge and compute their
summaries without entering the general recursive fixed-point loop.

This behavior MUST:

- preserve the same exact semantic contract for return types and local call targets;
- rely only on already stabilized out-of-SCC summaries plus the current routine body;
- keep self-recursive singleton SCCs off the fast path.

#### Scenario: Singleton non-recursive local routine resolves without recursive fixed-point

- **GIVEN** one local routine belongs to an SCC of size `1`
- **AND** that SCC has no self-edge
- **AND** callees outside the SCC are already stabilized by reverse-topological processing
- **WHEN** canonical semantic-facts materialization computes local-function summaries
- **THEN** the runtime computes that routine summary in one bounded pass
- **AND** it does not enter the general recursive fixed-point loop for that SCC
- **AND** the resulting summary remains equivalent to the exact semantic contract

#### Scenario: Self-recursive singleton stays on the convergence path

- **GIVEN** one local routine belongs to an SCC of size `1`
- **AND** that routine calls itself, so the SCC has a self-edge
- **WHEN** canonical semantic-facts materialization computes local-function summaries
- **THEN** the singleton fast path does not apply
- **AND** the routine summary is still derived through a convergence-safe recursive path

### Requirement: Recursive local-summary SCC solving MUST iterate SCC-locally rather than rebuilding file-wide snapshots

The system MUST, when canonical semantic-facts materialization solves a recursive local-routine
SCC, preserve a stable base view for out-of-SCC summaries and restrict per-iteration rebuild work
to the active SCC overlay rather than rebuilding a full-file local-summary snapshot.

This behavior MUST:

- let in-SCC lookups observe the latest current-SCC overlay values;
- let out-of-SCC lookups observe stable already-finalized summaries;
- avoid cloning, rebuilding, or remapping unrelated out-of-SCC summaries per SCC or per iteration
  under a helper that is only nominally called `base`;
- preserve deterministic ordering and convergence behavior for recursive SCCs.

#### Scenario: Recursive SCC iterations reuse stable out-of-SCC summaries

- **GIVEN** a file contains one recursive local-routine SCC and many unrelated local routines
- **WHEN** the runtime iterates that SCC to convergence
- **THEN** each iteration reuses stable summaries outside the active SCC from a base lookup
- **AND** only the active SCC overlay participates in per-iteration rebuild work
- **AND** the runtime does not rebuild a full-file local-summary snapshot on each iteration

### Requirement: Representative save-follow-up evidence MUST expose local-summary convergence attribution

The system MUST export low-cardinality local-summary convergence attribution for representative
same-file save-follow-up evidence whenever canonical semantic diagnostics report
`local_function_summaries` cost.

This evidence MUST include at least:

- total `local_function_summaries` latency;
- `prep`, `fixed_point`, `snapshot_build`, and `body_infer` subphases;
- `function_count`, `scc_count`, and fixed-point iteration count;
- `singleton_fast_path_count` and `recursive_scc_count`.

#### Scenario: Representative report distinguishes singleton fast-path wins from recursive residual

- **GIVEN** a representative large-module same-file save-follow-up exports canonical semantic
  diagnostics evidence
- **WHEN** `local_function_summaries` remains visible in that report
- **THEN** the report includes local-summary convergence attribution and bounded workload counts
- **AND** an operator can distinguish singleton fast-path wins from remaining recursive-SCC work

### Requirement: Exact same-version `program_lowering` MUST reuse unchanged lowering units for local same-file edits when safe

The system MUST derive a conservative lowering-reuse plan for exact same-version ready-snapshot
assembly from the previous exact ready state and the current changed ranges.
When that plan proves that some lowering units are unchanged, the runtime MUST reuse them instead
of rebuilding the entire lowering region.

This behavior MUST:

- stay bound to the exact `(file_id, requested_version, text_hash)` target;
- support reuse of unchanged top-level lowering units and bounded body-local reuse of unchanged
  sibling statement windows when soundness can be proven;
- rebuild any lowering region whose invalidation boundary cannot be proven safely;
- preserve exact same-version semantics, latest-wins supersession, and truthful cancellation /
  retarget behavior.

#### Scenario: Local edit inside one large callable body reuses unchanged lowering units

- **GIVEN** the previous same-file revision already has an exact ready snapshot
- **AND** the new revision changes only a bounded local region inside one large callable body
- **WHEN** exact ready-snapshot assembly builds the still-current target
- **THEN** the runtime reuses unchanged lowering units outside the invalidated region
- **AND** the exact path does not rebuild the whole file or whole body solely because one local
  edit occurred

#### Scenario: Ambiguous invalidation falls back to rebuild instead of stale reuse

- **GIVEN** a same-file edit touches or may affect a lowering boundary whose reuse soundness is not
  proven
- **WHEN** the runtime derives the exact lowering-reuse plan
- **THEN** the affected region is rebuilt fail-closed
- **AND** the system does not publish stale exact artifacts by guessing that reuse is safe

### Requirement: Exact `program_lowering` reuse MUST remain observable on representative load

The system MUST export operator-facing evidence showing how much exact `program_lowering` work was
reused versus rebuilt for one traced target on representative large-module same-file churn.
Acceptance for this change MUST prove reduced exact lowering work rather than only reduced wall-clock
latency with no visibility into what changed.

This behavior MUST:

- keep reuse-versus-rebuild evidence tied to one exact traced target and save cycle;
- expose a truthful reuse-plan outcome for the operator-facing trace or metrics snapshot;
- expose bounded summaries of reused and rebuilt lowering work for the exact path;
- preserve truthful dominant checkpoint and timeout attribution for the same traced target.

#### Scenario: Representative follow-up reports reduced exact lowering work with reuse evidence

- **GIVEN** a representative large-module same-file save follow-up exercises the exact path after
  the lowering-reuse change
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the evidence shows both the exact `program_lowering` residual and the reuse-versus-rebuild
  breakdown for that traced target
- **AND** operators can distinguish "less work was rebuilt" from "the system merely waited longer
  or relabeled the same hotspot"

#### Scenario: Full rebuild remains truthful when reuse does not qualify

- **GIVEN** the lowering-reuse plan decides that the current exact target must rebuild the affected
  region completely
- **WHEN** observability exports the traced follow-up result
- **THEN** the reuse-versus-rebuild evidence truthfully reports that reuse did not qualify
- **AND** dominant checkpoint and timeout attribution remain coherent for that full-rebuild path

### Requirement: Same-file `didSave` heavy follow-up MUST stop treating `shadow_state` as the steady-state terminal path once a bounded exact producer is still current

After `save_fastlane` already published the same-version first refresh, the system MUST prefer a
still-current exact same-version ready-snapshot producer strongly enough that `shadow_state`
remains a truthful fallback rather than the steady-state terminal branch for bounded
`program_lowering` workloads.

This behavior MUST:

- preserve the existing bounded wait budgets as the primary latency envelope;
- keep the still-current exact producer on the hottest path once it has already entered bounded
  `ready_snapshot_assembly` / `program_lowering`;
- avoid branch selection or same-file churn policies that repeatedly starve the best exact
  candidate while it is still the latest valid producer for the save cycle;
- preserve latest-wins supersession, cancellation, and exact same-version guarantees.

#### Scenario: Still-current bounded exact producer publishes the heavy follow-up through `ready_artifacts`

- **GIVEN** `didSave` already completed the same-version `save_fastlane` first publish
- **AND** the heavy follow-up is waiting on a still-current exact same-version producer that is
  already inside bounded `program_lowering`
- **AND** no newer same-file revision or newer save cycle supersedes that target
- **WHEN** the representative same-file mixed profile continues under the existing bounded
  follow-up policy
- **THEN** the heavy follow-up publishes through `ready_artifacts`
- **AND** `shadow_state` is not the terminal branch for that save cycle

#### Scenario: Newer same-file target still supersedes the exact producer truthfully

- **GIVEN** the heavy follow-up is currently waiting on a bounded exact same-version producer
- **AND** a newer same-file revision or newer save cycle arrives before publish
- **WHEN** the runtime re-evaluates the still-current target
- **THEN** the older producer MAY be superseded, cancelled, or retargeted truthfully
- **AND** the system does not keep the older save cycle alive just to avoid a `shadow_state`
  fallback

### Requirement: Same-file ranged `didChange` MUST keep a parser-base-capable exact head close enough to `shadow_state`

The system MUST keep a parser-base-capable exact head close enough to the live `shadow_state` that
`ready_snapshot_lags_shadow_state` stops being the dominant steady-state explanation for
`fallback_reason=stale_parser_base` on representative large-module same-file churn profiles.

This behavior MUST:

- remain bound to the exact `(file_id, requested_version, text_hash)` target;
- prefer advancing one still-current exact head or bounded recovery/prime path over repeatedly
  spawning parse workers that are predictably retargeted during `parse_exec`;
- preserve truthful fallback when a matching parser base still cannot be proven;
- preserve latest-wins semantics and MUST NOT reuse stale parser-base state for a newer revision.

#### Scenario: Representative ranged churn advances or recovers a parser-base-capable head before defaulting to `stale_parser_base`

- **GIVEN** same-file ranged `didChange` churn has advanced `shadow_state` beyond the latest ready
  exact head
- **AND** the next ranged revision would otherwise report
  `fallback_reason=stale_parser_base` with root cause `ready_snapshot_lags_shadow_state`
- **WHEN** the runtime chooses the next exact build / recovery path for that revision
- **THEN** it first advances or recovers a parser-base-capable exact head for the newest still-current target
- **AND** the newest ranged `didChange` does not default immediately to `stale_parser_base` solely
  because the old ready head lagged behind `shadow_state`

#### Scenario: Truthful fallback remains when no matching parser base can be proven

- **GIVEN** same-file ranged churn still cannot prove a matching parser base for the newest
  still-current revision after the bounded freshness / recovery path is exhausted
- **WHEN** the runtime finalizes the parse path for that revision
- **THEN** it MAY still fall back truthfully through `stale_parser_base`
- **AND** observability preserves that the bounded freshness / recovery path was attempted and
  exhausted for the same exact target

### Requirement: Same-version `didSave` follow-up MUST keep exact `program_lowering` bounded on the save-critical path

The system MUST treat exact same-version `program_lowering` as a bounded save-critical region
whenever `didSave` heavy follow-up is waiting on a still-current exact producer. The runtime MUST
not require a single monolithic lowering span to complete before save-critical promotion,
supersession checks, or the first publishable exact ready snapshot decision can take effect.

This behavior MUST:

- introduce bounded cooperative lowering checkpoints that the runtime can observe while exact
  lowering is still in progress;
- derive those checkpoints from actual lowering progress units (for example declaration, body, or
  bounded child batches) rather than only from wall-clock polling around one opaque lowering call;
- preserve exact same-version semantics for the produced ready snapshot;
- preserve truthful supersession / retarget behavior when a newer same-file revision or newer save
  cycle overtakes the current target.

#### Scenario: Save-critical exact producer advances through bounded lowering checkpoints

- **GIVEN** `didSave` heavy follow-up is waiting on an exact still-current same-version producer
- **AND** the producer is inside `program_lowering`
- **WHEN** runtime observes the next bounded lowering checkpoint
- **THEN** save-critical promotion and timeout attribution can react at that checkpoint
- **AND** the producer is not forced to remain invisible inside one monolithic lowering span

#### Scenario: Newer same-file target supersedes the bounded lowering producer

- **GIVEN** an exact same-version producer is already inside bounded `program_lowering`
- **AND** a newer same-file revision or newer save cycle arrives
- **WHEN** the producer reaches the next bounded lowering checkpoint
- **THEN** the producer MAY terminate or retarget truthfully instead of publishing stale output
- **AND** the system does not relax exactness rules for the superseded target

### Requirement: Exact `program_lowering` attribution MUST remain internally coherent for one traced target

The system MUST export target-coherent, internally coherent conversion attribution whenever exact
same-version ready-snapshot work is in or times out inside `program_lowering`. Operator-facing
observability MUST not emit one diagnostics-save trace whose aggregate `program_conversion` timing
contradicts its own bounded conversion slices.

This attribution MUST:

- remain tied to the exact current `(file_id, requested_version, text_hash)` target and
  `save_cycle_sequence`;
- merge or replace conversion attribution as one target-coherent tuple rather than as independent
  per-field maxima gathered from multiple probe snapshots;
- guarantee that exported `program_conversion_ms` is absent or greater than or equal to every
  constituent conversion slice present in the same trace;
- prevent stale aggregate conversion timing from one traced target or probe snapshot from leaking
  into another target's final follow-up trace;
- keep dominant checkpoint identity and dominant duration derived from the same target-coherent
  attribution view as the exported aggregate and bounded slice fields;
- preserve the higher-level truthful distinction between parse timeout, publish/apply blocker, and
  fallback-to-`shadow_state`.

#### Scenario: Timeout inside `program_lowering` reports coherent aggregate and slice timings

- **GIVEN** `didSave` follow-up times out while the exact same-version producer is still inside
  `program_lowering`
- **WHEN** diagnostics save timeline and incident bundle are exported
- **THEN** the exported evidence names the dominant lowering checkpoint truthfully
- **AND** `program_conversion_ms` is absent or greater than or equal to the reported
  `program_lowering_ms` and `publishable_artifact_packaging_ms`

#### Scenario: Repeated follow-up probe snapshots do not produce a self-contradictory final trace

- **GIVEN** the same `didSave` cycle records multiple follow-up probe snapshots while exact work is
  still moving through bounded conversion checkpoints
- **WHEN** diagnostics save timeline finalizes the operator-facing trace
- **THEN** the final trace keeps conversion aggregate, bounded slices, and dominant checkpoint
  coherent with one traced target
- **AND** the final trace does not merge stale aggregate timing with fresher per-slice maxima from
  another target state

### Requirement: Same-file didChange current-revision handoff registers ahead of full handler work (MUST)
Система MUST регистрировать current-revision `SetFile` handoff и публиковать same-file ingress
token для `(file_id, V)` через минимальный ingress fast lane после того, как same-file
`textDocument/didChange` для requested revision `V` уже принят и декодирован и сервер может
вычислить canonical updated text для этого change, но раньше, чем delayed full-handler work
(`lsp_did_change`, parse-snapshot scheduling, diagnostics scheduling или другой same-file
auxiliary work) сможет seconds-scale удерживать later completion для того же файла.

Этот fast lane MUST:

- обновлять `latest_received` и same-file shadow state именно тем текстом, который принят для
  `didChange` revision `V`;
- публиковать same-file ingress token только после того, как current-revision handoff для
  `(file_id, V)` действительно зарегистрирован;
- сохранять latest-wins и out-of-order semantics для same-file revisions;
- не допускать, чтобы downstream handler path double-apply-ил тот же `SetFile` или публиковал
  более сильную readiness semantics, чем реально был зарегистрированный handoff.

#### Scenario: Later completion no longer waits for full didChange handler entry
- **GIVEN** same-file `didChange` для revision `V` уже достиг server ingress
- **AND** сервер уже может вычислить canonical updated text для этого change
- **AND** full `lsp_did_change` handler work для той же notification ещё не завершилось
- **WHEN** позже приходит completion request для того же файла
- **THEN** completion MAY ждать truthful current-revision handoff для revision `V`
- **AND** completion MUST NOT spend seconds-scale same-file wait только потому, что
  `didChange` ещё не достиг full handler entry или его later auxiliary stages

#### Scenario: Dispatcher bookkeeping alone does not publish same-file freshness
- **GIVEN** same-file `didChange` для revision `V` уже создал barrier-owner или другое transport
  bookkeeping
- **AND** current-revision handoff для `(file_id, V)` ещё не зарегистрирован
- **WHEN** оператор читает authoritative completion trace
- **THEN** same-file ingress token для revision `V` остаётся не опубликован
- **AND** система не считает later same-file completion wait-free только по факту раннего
  dispatcher bookkeeping

#### Scenario: Superseded older didChange cannot re-publish stale same-file token
- **GIVEN** same-file `didChange` для revision `V` уже in-flight на fast lane
- **AND** затем приходит более новая revision `V+1` для того же файла
- **WHEN** latest-wins semantics выбирают текущую authoritative revision
- **THEN** older revision `V` MUST NOT publish or re-publish a same-file ingress token, который
  может задержать или исказить completion для `V+1`
- **AND** later same-file completion ждёт только ту revision, которая остаётся authoritative

### Requirement: Representative mixed-load evidence fails on post-didChange handoff lag (MUST)
Representative same-file mixed-load validation для крупного модуля MUST завершаться ошибкой, если
later completion всё ещё проводит seconds-scale время в `completion_barrier_wait_ms` или
`same_file_ingress_token_wait_ms` после того, как earlier same-file `didChange` уже наблюдался на
server ingress для требуемой revision и positive client/output-side waits не объясняют outlier.

Checked-in evidence для этого gate MUST сохранять хотя бы один correlation slice, который
показывает:

- requested revision completion trace;
- barrier owner revision, если owner присутствует;
- когда same-file handoff/token стал observable для этой revision.

#### Scenario: Live gate fails when handoff publication still lags after didChange ingress
- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** same-file `didChange` для requested revision уже наблюдался на server ingress до later
  completion trace
- **WHEN** measured completion sample всё ещё тратит seconds-scale время в
  `completion_barrier_wait_ms` или `same_file_ingress_token_wait_ms`
- **THEN** representative gate завершается ошибкой
- **AND** regression не маскируется под generic client ingress, output handoff или cold
  query-body latency

#### Scenario: Worst outlier evidence preserves same-file revision ownership
- **GIVEN** representative same-file mixed-load profile уже поймал worst completion outlier
- **WHEN** оператор читает checked-in evidence
- **THEN** evidence сохраняет correlation slice c requested revision и barrier owner revision,
  когда owner доступен
- **AND** по evidence можно понять, когда same-file handoff/token стал observable для этой
  completion path

