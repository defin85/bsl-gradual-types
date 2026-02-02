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

### Requirement: `SemanticProgram.cfg` всегда присутствует в v2 snapshot (MUST)
Система MUST всегда включать CFG в v2 snapshot как `SemanticProgram.cfg = Some(ControlFlowGraph)`, даже если в файле нет исполняемых конструкций.

Минимальный CFG в таком случае MUST содержать:
- узлы `Entry` и `Exit`,
- ребро `Entry -> Exit`.

#### Scenario: CFG присутствует для файлов без исполняемых конструкций
- **GIVEN** пользователь открывает `.bsl` файл, который содержит только объявления (или пустые тела процедур/функций)
- **WHEN** система строит v2 snapshot (IR) для hover/completion/diagnostics
- **THEN** `SemanticProgram.cfg` присутствует и содержит как минимум `Entry -> Exit`
- **AND** flow-sensitive операции (narrowing / null-safety) не требуют fallback на отсутствие CFG

### Requirement: Привязка “позиция → CFG узел” детерминирована и bias-aware (MUST)
Система MUST вычислять spans CFG узлов на основе структурной информации tree-sitter (через Syntax AST), а не строковых эвристик поиска ключевых слов.

Для всех CFG узлов вида `CfgNodeKind::Conditional` (включая `If` и `TryExcept`) система MUST:
- иметь span “header”, который НЕ покрывает тело веток;
- иметь отдельные узлы-маркеры со spans тела веток (`then/else`, `try/except`), создаваемые даже для пустых веток;
- обеспечивать, что `node_at_byte_offset(offset, bias)` выбирает “самый специфичный” узел по span и поэтому может детерминированно отличать header от body.

#### Scenario: Вложенный `Иначе` внутри else-ветки не ломает mapping веток
- **GIVEN** код содержит `Если ... Тогда ... Иначе ...` и внутри else-ветки есть вложенный `Если ... Иначе ... КонецЕсли`
- **WHEN** IDE запрашивает hover/completion в позиции внутри тела внешней then-ветки и внутри тела внешней else-ветки
- **THEN** система выбирает соответствующие marker-узлы тела внешних веток (а не header и не вложенные ветки)
- **AND** результат детерминирован при повторных запросах

#### Scenario: `Попытка/Исключение` имеет корректные spans веток
- **GIVEN** код содержит `Попытка ... Исключение ... КонецПопытки` (включая пустые тела)
- **WHEN** IDE запрашивает контекст по позиции внутри try-body и внутри except-body
- **THEN** система выбирает marker-узлы соответствующих веток по spans

### Requirement: Flow-sensitive null-safety учитывает null-check в заголовках циклов (MUST)
Система MUST учитывать null-check условия в заголовках циклов (например, `Пока x <> Null Цикл`) при выполнении flow-sensitive null-safety анализа в v2 pipeline.

#### Scenario: Null-check в `Пока` подавляет warning в теле
- **GIVEN** переменная `x` потенциально nullable
- **WHEN** код содержит цикл `Пока x <> Null Цикл ... x.Method() ... КонецЦикла`
- **THEN** flow-sensitive null-safety не выдаёт предупреждение о возможном Null‑dereference для `x.Method()` внутри тела цикла

### Requirement: Completion на `.` в then-ветке после `ТипЗнч`-guard использует narrowing (MUST)
Система MUST обеспечивать, что completion на позиции сразу после `.` внутри then-ветки условия `ТипЗнч(x) = Тип("...")` использует flow-sensitive narrowing и выбирает контекст ветки (а не fallback).

#### Scenario: Completion в then-ветке после `ТипЗнч`-guard возвращает members суженного типа
- **GIVEN** код содержит:
  - `x` объявлена/инициализирована без точного типа,
  - `Если ТипЗнч(x) = Тип("ТаблицаЗначений") Тогда x. ... КонецЕсли`
- **WHEN** IDE запрашивает completion в позиции сразу после `x.`
- **THEN** среди результатов присутствует член `Колонки` (как member типа `ТаблицаЗначений`)
- **AND** тест LSP уровня интеграции фиксирует этот сценарий

