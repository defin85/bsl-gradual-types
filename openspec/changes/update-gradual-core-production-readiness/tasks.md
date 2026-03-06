## 1. Contract
- [ ] 1.1 Зафиксировать, что shared resolved contract MUST first-class выражать structural members snapshot-local уровня для typed `Структура` и typed-row.
- [ ] 1.2 Зафиксировать, что structural member entry содержит минимум:
  - [ ] canonical member name
  - [ ] stable member identity
  - [ ] member type
  - [ ] certainty
  - [ ] source span / source location
- [ ] 1.3 Зафиксировать, что generic/base-type-only representation недостаточна как shared truth для structural members.
- [ ] 1.4 Зафиксировать, что completion / hover / type-at-position / diagnostics / MCP / LSP используют один semantic resolved path или thin adapters поверх него.
- [ ] 1.5 Зафиксировать future readiness gate для dev-workflow: change нельзя считать complete, если критический follow-up backlog по его MUST-требованиям остаётся открытым.

## 2. Design
- [ ] 2.1 Описать варианты формы shared structural contract:
  - [ ] расширение `TypeResolution`
  - [ ] эквивалентный explicit shared sidecar contract
- [ ] 2.2 Выбрать migration strategy для consumer-local веток и исключений completion.
- [ ] 2.3 Описать exact acceptance matrix, которая доказывает semantic equivalence между consumers, а не только smoke/parity.
- [ ] 2.4 Описать governance path для согласования OpenSpec status, traceability matrix и Beads backlog.

## 3. Validation
- [ ] 3.1 Подготовить traceability `Requirement -> Future Code Area -> Required Test Class`.
- [ ] 3.2 Прогнать `openspec validate update-gradual-core-production-readiness --strict --no-interactive`.
