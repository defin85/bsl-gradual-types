## 1. Specification
- [ ] 1.1 Добавить в `dev-workflow` требование ADR gate для архитектурно-значимых/perf-critical изменений.
- [ ] 1.2 Добавить в `dev-workflow` требование doc-first non-MVP контракта с acceptance matrix до имплементации.
- [ ] 1.3 Добавить в `dev-workflow` требование test-first цикла для backend/runtime behavioral changes.
- [ ] 1.4 Добавить в `dev-workflow` требование protected acceptance assets с fail-closed policy.
- [ ] 1.5 Добавить в `dev-workflow` требование perf evidence merge-gate (`latency + allocations + lock contention`).
- [ ] 1.6 Добавить в `bsl-intellisense-v2` resource budget requirement для интерактивного completion.
- [ ] 1.7 Добавить в `bsl-intellisense-v2` low-cardinality resource observability requirement.

## 2. Design And Tooling
- [ ] 2.1 Утвердить ADR template и критерии "architecturally significant/perf-critical change".
- [ ] 2.2 Определить protected-assets manifest (какие тесты/контракты/baselines считаются immutable в implementation change).
- [ ] 2.3 Спроектировать format/version policy для resource baseline artifacts и отчётов gate.
- [ ] 2.4 Зафиксировать ownership: кто утверждает ADR, кто владелец perf budgets, кто владелец protected-assets policy.

## 3. Implementation Rollout
- [ ] 3.1 Реализовать process-gates (ADR/doc-first/protected-assets) как автоматические проверки в CI/локальном workflow.
- [ ] 3.2 Добавить instrumentation completion hot path для allocation/lock metrics.
- [ ] 3.3 Реализовать extended perf gate с deterministic report и fail-closed verdict.
- [ ] 3.4 Провести staged rollout: warning-only на первом цикле, затем blocking после фиксации baseline.

## 4. Validation
- [ ] 4.1 `openspec validate add-performance-first-ai-engineering-guardrails --strict --no-interactive`.
- [ ] 4.2 Dry-run на репрезентативных профилях (`small`, `large`, `churn`) и подтверждение воспроизводимости отчётов.
- [ ] 4.3 Review с владельцами `analysis-v2`, `runtime`, `LSP`, и process ownership.

## Dependencies / Parallelism
- [ ] D1 Пункты 2.1 и 2.2 блокируют 3.1.
- [ ] D2 Пункт 2.3 блокирует 3.3.
- [ ] D3 Пункт 3.2 может выполняться параллельно с 3.1 после завершения 2.4.
