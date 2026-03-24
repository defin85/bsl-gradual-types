## 1. Specification
- [x] 1.1 Добавить в `bsl-intellisense-v2` контракт split-prepare для completion между lightweight current-revision path и exact stateful path.
- [x] 1.2 Уточнить freshness policy так, чтобы member-access completion по default path не требовал full `snapshot_with_deps` как обязательный prereq для `head_hit`.
- [x] 1.3 Зафиксировать в spec, что representative real-module gate должен отдельно покрывать `same-revision warm` и `revision-churn`, извлекать route attribution и ловить regressions effectively exact-first completion.

## 2. Design
- [x] 2.1 Зафиксировать root-cause reasoning: residual latency теперь сидит в generic prepare boundary, а не только в transport или post-handoff readiness.
- [x] 2.2 Зафиксировать новый application boundary для completion (`head-ready` / `exact-ready` / `not-ready`) как feature-specific request-scoped contract без утечки writer-owned runtime state.
- [x] 2.3 Зафиксировать safe contents lightweight current-revision context и запрет long-lived shared `AnalysisV2` как feature boundary.
- [x] 2.4 Описать, как exact path продолжает использовать существующий `PreparedOperationSnapshot` без регрессии для `hover`, `definition`, `signatureHelp` и `type-at-position`.
- [x] 2.5 Описать rollout order, supersession и cancellation semantics между lightweight head path и exact upgrade.
- [x] 2.6 Зафиксировать delivery gaps после implementation review: до закрытия change нужно сузить public lightweight boundary, довести shipped gate/evidence до `p37` + `p38` и оформить checked-in review outcome.

## 3. Validation
- [x] 3.1 Провалидировать change: `openspec validate refactor-completion-prepare-lightweight-exact-split --strict --no-interactive`.
- [ ] 3.2 Зафиксировать checked-in outcome архитектурного review change с владельцами runtime/LSP completion boundary и подтвердить, что change не требует detached immutable snapshot как prereq.

## 4. Implementation Closure
- [x] 4.1 Сузить public lightweight completion boundary до узкого request-scoped payload и убрать `AnalysisV2` как внешний carrier completion first-response API.
- [ ] 4.2 Довести shipped representative gate/evidence path: default workflow, CI, scripts, docs и checked-in reports должны запускать и подтверждать оба обязательных real-module профиля `p37 same-revision warm` и `p38 revision-churn/post-handoff readiness`.

> Зависимость между change: `4.1` можно закрывать параллельно, но `4.2` опирается на producer-side readiness invariants из `refactor-current-revision-readiness-fast-lane`. Финальный shipped gate/evidence для split-prepare нельзя считать завершённым, пока runtime/gate задачи `gy9c.13` и `gy9c.14` не зелёные.
