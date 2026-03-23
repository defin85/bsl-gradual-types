## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` контракт detached immutable current-revision head snapshot как canonical derived read model для completion first-response path.
- [ ] 1.2 Зафиксировать publication, invalidation и supersession semantics для snapshot, keyed по `(file_id, file_version, deps_id, settings_id)`.
- [ ] 1.3 Уточнить latency/read-path contract так, чтобы после publication detached head snapshot first response не зависел от writer-owned mutable runtime state.

## 2. Design
- [ ] 2.1 Описать snapshot schema и bounded contents detached head read model.
- [ ] 2.2 Явно зафиксировать запрет на использование long-lived shared `AnalysisV2` как detached snapshot substitute.
- [ ] 2.3 Описать publication flow, invalidation по deps/settings/version и latest-wins supersession semantics.
- [ ] 2.4 Описать consumer API для completion и границу между detached head snapshot и exact stateful prepare.
- [ ] 2.5 Описать rollout order и relationship с `refactor-completion-prepare-lightweight-exact-split`.
- [ ] 2.6 Зафиксировать observability и representative gate для detached read path.

## 3. Validation
- [ ] 3.1 Провалидировать change: `openspec validate refactor-current-revision-head-detached-snapshot --strict --no-interactive`.
- [ ] 3.2 Провести архитектурный review change с владельцами analysis-v2/runtime/LSP и подтвердить, что detached snapshot действительно является immutable published read model, а не disguised runtime snapshot.
