## 1. Specification
- [ ] 1.1 Добавить в `bsl-intellisense-v2` контракт current-revision readiness fast lane для `applied_version` advance и `CompletionHeadArtifact` publish после document-sync handoff.
- [ ] 1.2 Уточнить churn-aware completion requirement так, чтобы `prepare_timeout@wait_for_file_version` после same-file handoff считался regression readiness scheduler.
- [ ] 1.3 Уточнить churn-aware completion requirement так, чтобы `exact_deadline` при уже достигнутом current `observed_file_version`, но `head_ready=false`, считался regression head-readiness fast lane.
- [ ] 1.4 Расширить representative real-module gate профилем `post-handoff readiness` и закрепить pass/fail budgets по `wait_for_file_version_runtime.queue_wait_ms` и failure conditions по existing authoritative fields.

## 2. Design
- [ ] 2.1 Зафиксировать root-cause reasoning по incident bundle `2026-03-23T08:03:23Z` и показать, что bottleneck сместился из transport ingress в current-revision readiness path.
- [ ] 2.2 Описать fast-lane split между `applied_version` / `CompletionHeadArtifact` readiness и slow exact/type-index/diagnostics enrich path.
- [ ] 2.3 Явно зафиксировать, что newest same-file readiness work может supersede older-revision background work, не нарушая latest-wins semantics.
- [ ] 2.4 Описать acceptance gate, который различает post-handoff apply backlog, post-apply head gap и slow exact-upgrade latency.
- [ ] 2.5 Зафиксировать отклонённые альтернативы: увеличение wait budget, stale fallback, pure concurrency uplift, exact-only head readiness.
- [ ] 2.6 Явно зафиксировать, что change не обещает ускорение full exact/type-index throughput вне контекста first current-revision response.

## 3. Validation
- [ ] 3.1 Провалидировать change: `openspec validate refactor-current-revision-readiness-fast-lane --strict --no-interactive`.
- [ ] 3.2 Провести review change с владельцами LSP/document-sync/runtime readiness path, используя bundles `2026-03-22T16:19:59Z` и `2026-03-23T08:03:23Z` как evidence.
