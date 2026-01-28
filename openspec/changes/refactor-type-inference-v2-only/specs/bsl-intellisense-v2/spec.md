## ADDED Requirements

### Requirement: v2 pipeline является единственным источником истины для вывода типов (MUST)
Система MUST использовать `bsl-analysis-v2` как единственный pipeline вывода типов для IDE‑функций (completion/hover/signatureHelp/definition/diagnostics).
Legacy‑пути вывода типов MUST быть удалены (не поддерживаются).

#### Scenario: Hover и completion используют один и тот же источник типовой информации
- **GIVEN** пользователь работает в IDE с `.bsl` файлом
- **WHEN** IDE запрашивает hover и completion в одной и той же позиции/контексте
- **THEN** ответы должны опираться на единый v2 snapshot и согласованный набор правил (без альтернативных inference путей)

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
