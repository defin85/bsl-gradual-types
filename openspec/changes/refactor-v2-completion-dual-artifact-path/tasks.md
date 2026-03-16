## 1. Spec and contract
- [ ] 1.1 Обновить `bsl-intellisense-v2` contract так, чтобы completion использовал canonical `head-or-exact-or-fail-closed`, а остальные interactive semantic операции сохранили exact-or-fail-closed.
- [ ] 1.2 Зафиксировать в дизайне lifecycle и invalidation для `CompletionHeadArtifact` и `ExactSemanticArtifact`.

## 2. Completion head artifact
- [ ] 2.1 Ввести `CompletionHeadArtifact` как отдельный canonical derived artifact из current-revision IR snapshot.
- [ ] 2.2 Ограничить первую фазу `CompletionHeadArtifact` member-access completion path, не расширяя его на `hover/definition/type-at-position/diagnostics`.
- [ ] 2.3 Добавить инвариантные тесты invalidation по `(file_version, deps_id, settings_id)`.

## 3. LSP/runtime orchestration
- [ ] 3.1 Перевести completion orchestration на `head-or-exact-or-fail-closed`.
- [ ] 3.2 Сохранить `hover`, `definition`, `signatureHelp`, `type-at-position` в exact-or-fail-closed режиме.
- [ ] 3.3 Сохранить стабильный `candidate_id` contract и покрыть tests на согласованность `head` ответа и exact `resolve`.

## 4. Scheduling and observability
- [ ] 4.1 Добавить waiter-aware orchestration для exact precompute одной revision вместо независимых конкурентных exact builds.
- [ ] 4.2 Разделить observability на head-hit, exact-hit, head-to-exact-upgrade и fail-closed-by-deadline.
- [ ] 4.3 Обновить completion timeline/metrics contract без high-cardinality drift.

## 5. Perf gates and validation
- [ ] 5.1 Обновить representative real-module gate (`p36`-style) так, чтобы первый warm completion на real module MUST быть `ok_non_empty`.
- [ ] 5.2 Зафиксировать budgets:
  - first-response completion head `p95 <= 150ms` на representative large module;
  - exact upgrade измеряется отдельно и не маскирует first-response availability.
- [ ] 5.3 Прогнать `openspec validate refactor-v2-completion-dual-artifact-path --strict --no-interactive` и приложить команды/артефакты acceptance.
