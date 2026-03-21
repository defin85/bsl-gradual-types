## 1. Spec and contract
- [x] 1.1 Обновить `bsl-intellisense-v2` contract так, чтобы completion использовал canonical `head-or-exact-or-fail-closed` именно под `revision-churn`, а остальные interactive semantic операции сохранили exact-or-fail-closed.
- [x] 1.2 Зафиксировать в дизайне lifecycle, invalidation и split `head-ready` / `exact-ready` semantics для `CompletionHeadArtifact` и `ExactSemanticArtifact`.

## 2. Completion head artifact
- [x] 2.1 Ввести `CompletionHeadArtifact` как отдельный canonical derived artifact из current-revision IR snapshot.
- [x] 2.2 Ограничить первую фазу `CompletionHeadArtifact` member-access completion path, не расширяя его на `hover/definition/type-at-position/diagnostics`.
- [x] 2.3 Добавить инвариантные тесты invalidation по `(file_version, deps_id, settings_id)`.
- [x] 2.4 Гарантировать, что публикация `CompletionHeadArtifact` для текущей revision не зависит от готовности `ExactSemanticArtifact` той же revision.

## 3. LSP/runtime orchestration
- [x] 3.1 Перевести completion orchestration на split prepare (`head-ready` перед `exact-ready`) и `head-or-exact-or-fail-closed`.
- [x] 3.2 Сохранить `hover`, `definition`, `signatureHelp`, `type-at-position` в exact-or-fail-closed режиме.
- [x] 3.3 Сохранить стабильный `candidate_id` contract и покрыть tests на согласованность `head` ответа и exact `resolve`.

## 4. Scheduling and observability
- [x] 4.1 Добавить debounce/coalesce + waiter-aware orchestration для exact precompute одной revision вместо независимых конкурентных exact builds на каждом `didChange`.
- [x] 4.2 Разделить observability на `prepare timeout`, `exact deadline`, head-hit, exact-hit и head-to-exact-upgrade.
- [x] 4.3 Обновить completion timeline/metrics contract без high-cardinality drift.

## 5. Perf gates and validation
- [x] 5.1 Обновить representative real-module gates так, чтобы:
  - `same-revision warm` probe подтверждал healthy steady-state;
  - `revision-churn` probe (`didChange -> completion` перед каждым measured sample) требовал current-revision `ok_non_empty`.
- [x] 5.2 Зафиксировать budgets:
  - first-response completion head под `revision-churn` `p95 <= 150ms` на representative large module;
  - same-revision warm измеряется отдельно и не должен регрессировать относительно head path;
  - exact upgrade измеряется отдельно и не маскирует first-response availability.
- [x] 5.3 Прогнать `openspec validate refactor-v2-completion-dual-artifact-path --strict --no-interactive` и приложить команды/артефакты acceptance.
