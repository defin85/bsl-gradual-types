## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` контракт split-prepare для completion между lightweight current-revision path и exact stateful path.
- [ ] 1.2 Уточнить freshness policy так, чтобы member-access completion по default path не требовал full `snapshot_with_deps` как обязательный prereq для `head_hit`.
- [ ] 1.3 Расширить representative real-module gate так, чтобы он проверял route attribution и доказывал lightweight first-response path отдельно от exact upgrade.

## 2. Design
- [ ] 2.1 Зафиксировать root-cause reasoning: residual latency теперь сидит в generic prepare boundary, а не только в transport или post-handoff readiness.
- [ ] 2.2 Описать новый application boundary для completion (`head-ready` / `exact-ready` / `not-ready`) без утечки writer-owned runtime state.
- [ ] 2.3 Явно определить safe contents lightweight current-revision context и запретить long-lived shared `AnalysisV2` как feature boundary.
- [ ] 2.4 Описать, как exact path продолжает использовать существующий `PreparedOperationSnapshot` без регрессии для `hover`, `definition`, `signatureHelp` и `type-at-position`.
- [ ] 2.5 Описать rollout order, supersession и cancellation semantics между lightweight head path и exact upgrade.
- [ ] 2.6 Зафиксировать acceptance evidence для representative real-module gate и synthetic regression tests.

## 3. Validation
- [ ] 3.1 Провалидировать change: `openspec validate refactor-completion-prepare-lightweight-exact-split --strict --no-interactive`.
- [ ] 3.2 Провести архитектурный review change с владельцами runtime/LSP completion boundary и подтвердить, что change не требует detached immutable snapshot как prereq.
