# ADR: governance gate for isolate-completion-pre-dispatch-ingress

## Status
accepted

## Change ID and Criticality
- change_id: `isolate-completion-pre-dispatch-ingress`
- change_criticality: `architectural`

## Context
Change добавляет новый transport-adapter boundary для truthful pre-dispatch attribution и отдельный scheduler ownership seam. Без change-local governance package этот change мог бы выглядеть как passable по prose review, даже если machine-readable gate фактически skipped бы validation.

## Options Considered
1. Оставить change без change-local governance package и надеяться на ручной review.
2. Добавить machine-readable governance package, acceptance matrix и evidence refs внутри change-root.

## Decision
Выбран вариант 2.

Для этого change gate должен fail closed, если отсутствует хотя бы один обязательный governance artifact, если evidence ref указывает вне change-root или если acceptance matrix не содержит явных pass/fail критериев.

## Budgets
- Governance budget: skip-path запрещён, missing artifact считается hard fail.
- Evidence budget: все required refs должны указывать на файлы внутри change-root.
- Acceptance budget: validation matrix обязан описывать явные pass/fail критерии для governance gate и representative mixed-load gate.

## Rollback
Если gate начнёт шуметь без оснований, изменение допускается только через новый approved OpenSpec change. Ослаблять validation silently нельзя.

## Owners and Approvers
- Analysis owner: truthful split между client ingress и server ingress задокументирован и проверяем.
- Runtime owner: scheduler isolation и cancellation semantics отражены в evidence.
- LSP owner: versioned completion timeline и mixed-load gate покрыты.
- Process owner: governance package локален для change и работает fail closed.
