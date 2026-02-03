## ADDED Requirements

### Requirement: Flow-sensitive результаты доступны во всех интерфейсах v2 и вычисляются только по запросу (MUST)
Система MUST предоставлять flow-sensitive результаты (type narrowing и null-safety) для IDE (LSP), Web API и MCP, используя только v2 snapshot/queries.

Flow-sensitive вычисления MUST выполняться только при явном включении флага/настройки (default: OFF), чтобы не ухудшать производительность по умолчанию.

#### Scenario: Flow-sensitive выключен по умолчанию
- **GIVEN** пользователь использует IDE/Web API/MCP без явного включения flow-sensitive режима
- **WHEN** выполняются hover/completion/diagnostics/type-at-position запросы
- **THEN** система не запускает flow-sensitive вычисления и возвращает результаты на основе базовых v2 queries (как и ранее)

#### Scenario: Flow-sensitive включён и влияет на hover/completion/diagnostics
- **GIVEN** пользователь явно включил flow-sensitive режим в IDE/Web API/MCP
- **WHEN** система отвечает на hover/completion/diagnostics запросы
- **THEN** ответы используют flow-sensitive результаты, вычисленные из v2 snapshot/queries (на основе CFG), и эти результаты согласованы между интерфейсами

### Requirement: v2 предоставляет корректный контракт “позиция → flow-sensitive тип” (MUST)
Система MUST иметь стабильный механизм получения flow-sensitive `TypeResolution` для byte offset позиции в документе, чтобы hover/completion/signatureHelp/definition могли использовать уточнённый тип в текущем control-flow контексте.

Реализация SHOULD быть локальной по области анализа (например, CFG-per-body), чтобы минимизировать стоимость вычислений.

#### Scenario: Type-at-position учитывает narrowing в then-ветке
- **GIVEN** переменная имеет широкий/nullable тип до условия
- **WHEN** курсор находится внутри then-ветки после type guard (например, `x <> Неопределено` / `ТипЗнч(x)=...`)
- **THEN** flow-sensitive `type-at-position` возвращает уточнённый тип (narrowed) для `x`

### Requirement: Null-safety diagnostics интегрированы в v2 diagnostics при включении (MUST)
Система MUST добавлять null-safety diagnostics, вычисленные на основе CFG и flow-sensitive контекста, в v2 diagnostics pipeline при включённом flow-sensitive режиме.

#### Scenario: Null-safety предупреждение появляется только при включённом режиме
- **GIVEN** код содержит потенциальный null dereference по CFG (receiver может быть null/undefined)
- **WHEN** запрашиваются diagnostics при включённом flow-sensitive режиме
- **THEN** система возвращает null-safety diagnostics
- **AND** при выключенном режиме эти diagnostics отсутствуют

