# ADR: readiness gate for update-gradual-core-production-readiness

## Status
accepted

## Change ID and Criticality
- change_id: `update-gradual-core-production-readiness`
- change_criticality: `architectural`

## Context
Change начинался как future-facing contract, но после `6mx.1`, `6mx.2`, `6mx.3`, `6mx.5`, `6mx.7` и `6mx.8`
получил прямое semantic evidence. Финальный риск оставался процессным: без machine-readable readiness gate
change можно было бы ошибочно трактовать как `complete` только по зелёному checklist и strict validation.

## Options Considered
1. Оставить checklist + review notes без machine-readable readiness status.
2. Ввести change-specific governance gate с explicit `partial/not_ready/complete` verdict и привязкой к Beads backlog.

## Decision
Выбран вариант 2.

Для этого change `complete` допустим только после одновременного выполнения всех условий:
- review verdict и traceability status находятся в success-state;
- критический follow-up backlog закрыт или есть approved superseding delivery path;
- governance artifacts проходят fail-closed validation.

До выполнения этих условий change обязан оставаться `partial` или `not_ready`.

## Budgets
- Readiness budget: optimistic `complete` запрещён при открытом critical backlog.
- Evidence budget: acceptance matrix, test-first refs, dependency checks, ownership sign-off и readiness status обязательны.
- Traceability budget: `Requirement -> Code -> Test` must be refreshed before final closure verdict.

## Rollback
Если gate блокирует change ошибочно, policy меняется только новым approved change или обновлённым ADR.
Удаление readiness artifacts или перевод status в `complete` без evidence не считается допустимым rollback path.

## Owners and Approvers
- Analysis owner: shared structural contract и `analysis-v2` evidence.
- Runtime owner: unified resolved path и completion/runtime ownership guarantees.
- LSP owner: LSP/MCP/Web adapter parity и exact acceptance surface.
- Process owner: OpenSpec governance, readiness status и backlog alignment.
