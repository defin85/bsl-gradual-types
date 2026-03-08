## 1. Contract
- [x] 1.1 Зафиксировать, что shared resolved contract MUST first-class выражать structural members snapshot-local уровня для typed `Структура` и typed-row.
- [x] 1.2 Зафиксировать, что structural member entry содержит минимум:
  - [x] canonical member name
  - [x] stable member identity
  - [x] member type
  - [x] certainty
  - [x] source span / source location
- [x] 1.3 Зафиксировать, что generic/base-type-only representation недостаточна как shared truth для structural members.
- [x] 1.4 Зафиксировать, что completion / hover / type-at-position / diagnostics / MCP / LSP используют один semantic resolved path или thin adapters поверх него.
- [x] 1.5 Зафиксировать future readiness gate для dev-workflow: change нельзя считать complete, если критический follow-up backlog по его MUST-требованиям остаётся открытым.

## 2. Design
- [x] 2.1 Описать варианты формы shared structural contract:
  - [x] расширение `TypeResolution`
  - [x] эквивалентный explicit shared sidecar contract
- [x] 2.2 Выбрать migration strategy для consumer-local веток и исключений completion.
- [x] 2.3 Описать exact acceptance matrix, которая доказывает semantic equivalence между consumers, а не только smoke/parity.
- [x] 2.4 Описать governance path для согласования OpenSpec status, traceability matrix и Beads backlog.

## 3. Validation
- [x] 3.1 Подготовить traceability `Requirement -> Future Code Area -> Required Test Class`.
- [x] 3.2 Прогнать `openspec validate update-gradual-core-production-readiness --strict --no-interactive`.
