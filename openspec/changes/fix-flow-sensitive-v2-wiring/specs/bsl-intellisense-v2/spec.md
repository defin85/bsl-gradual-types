## ADDED Requirements

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

