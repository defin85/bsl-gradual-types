## ADDED Requirements

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
