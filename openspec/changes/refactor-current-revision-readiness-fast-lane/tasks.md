## 1. Specification
- [x] 1.1 Добавить в `bsl-intellisense-v2` контракт current-revision readiness fast lane для `applied_version` advance и `CompletionHeadArtifact` publish после document-sync handoff.
- [x] 1.2 Уточнить churn-aware completion requirement так, чтобы `prepare_timeout@wait_for_file_version` после same-file handoff считался regression readiness scheduler.
- [x] 1.3 Уточнить churn-aware completion requirement так, чтобы `exact_deadline` при уже достигнутом current `observed_file_version`, но `head_ready=false`, считался regression head-readiness fast lane.
- [x] 1.4 Расширить representative real-module gate профилем `post-handoff readiness` и закрепить pass/fail budgets по `wait_for_file_version_runtime.queue_wait_ms` и failure conditions по existing authoritative fields.

## 2. Design
- [x] 2.1 Зафиксировать root-cause reasoning по incident bundle `2026-03-23T08:03:23Z` и показать, что bottleneck сместился из transport ingress в current-revision readiness path.
- [x] 2.2 Описать fast-lane split между `applied_version` / `CompletionHeadArtifact` readiness и slow exact/type-index/diagnostics enrich path.
- [x] 2.3 Явно зафиксировать, что newest same-file readiness work может supersede older-revision background work, не нарушая latest-wins semantics.
- [x] 2.4 Описать acceptance gate, который различает post-handoff apply backlog, post-apply head gap и slow exact-upgrade latency.
- [x] 2.5 Зафиксировать отклонённые альтернативы: увеличение wait budget, stale fallback, pure concurrency uplift, exact-only head readiness.
- [x] 2.6 Явно зафиксировать, что change не обещает ускорение full exact/type-index throughput вне контекста first current-revision response.

## 3. Validation
- [ ] 3.1 Провалидировать change: `openspec validate refactor-current-revision-readiness-fast-lane --strict --no-interactive`.
- [ ] 3.2 Провести review change с владельцами LSP/document-sync/runtime readiness path, используя bundles `2026-03-22T16:19:59Z` и `2026-03-23T08:03:23Z` как evidence.
- [ ] 3.3 Довести runtime path: completion consumer должен читать current-revision readiness через operation-aware fast lane, а не через background snapshot/readiness path.
- [ ] 3.4 Довести acceptance: shipped representative gate, workflow и docs должны проверять post-handoff readiness budgets/failure conditions на live LSP path.
- [ ] 3.5 Синхронизировать traceability, task state и validation evidence между OpenSpec, Beads и checked-in артефактами.

> Этот change является producer-side prerequisite для финального shipped gate/evidence closure в `refactor-completion-prepare-lightweight-exact-split`: не закрывать split-prepare acceptance как завершённый, пока `3.3` и `3.4` здесь не зелёные.
