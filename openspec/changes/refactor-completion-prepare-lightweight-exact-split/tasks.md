## 1. Specification
- [x] 1.1 Добавить в `bsl-intellisense-v2` контракт split-prepare для completion между lightweight current-revision path и exact stateful path.
- [x] 1.2 Уточнить freshness policy так, чтобы member-access completion по default path не требовал full `snapshot_with_deps` как обязательный prereq для `head_hit`.
- [ ] 1.3 Довести representative real-module gate до полного spec coverage: по default shipped path должны отдельно запускаться и блокировать `same-revision warm`, и `revision-churn` профили, а не только `revision-churn`.

## 2. Design
- [x] 2.1 Зафиксировать root-cause reasoning: residual latency теперь сидит в generic prepare boundary, а не только в transport или post-handoff readiness.
- [ ] 2.2 Довести новый application boundary для completion (`head-ready` / `exact-ready` / `not-ready`) до узкого request-scoped payload без публичной утечки широкого `AnalysisV2` через lightweight contract.
- [ ] 2.3 Довести safe contents lightweight current-revision context: убрать broad `AnalysisV2` из публичного lightweight boundary и сохранить запрет на long-lived shared `AnalysisV2` как feature boundary.
- [x] 2.4 Описать, как exact path продолжает использовать существующий `PreparedOperationSnapshot` без регрессии для `hover`, `definition`, `signatureHelp` и `type-at-position`.
- [x] 2.5 Описать rollout order, supersession и cancellation semantics между lightweight head path и exact upgrade.
- [ ] 2.6 Довести acceptance evidence: checked-in gating path должен включать wired `p37` + `p38`, а readiness assets и reports должны подтверждать оба обязательных real-module профиля.

## 3. Validation
- [x] 3.1 Провалидировать change: `openspec validate refactor-completion-prepare-lightweight-exact-split --strict --no-interactive`.
- [ ] 3.2 Зафиксировать checked-in outcome архитектурного review change с владельцами runtime/LSP completion boundary и подтвердить, что change не требует detached immutable snapshot как prereq.
