## 1. Specification
- [ ] 1.1 Добавить в `dev-workflow` требование ADR gate для архитектурно-значимых/perf-critical изменений.
- [ ] 1.2 Добавить в `dev-workflow` требование doc-first non-MVP контракта с acceptance matrix до имплементации.
- [ ] 1.3 Добавить в `dev-workflow` требование test-first цикла для backend/runtime behavioral changes.
- [ ] 1.4 Добавить в `dev-workflow` требование protected acceptance assets с fail-closed policy.
- [ ] 1.5 Добавить в `dev-workflow` требование perf evidence merge-gate (`latency + allocations + lock contention`).
- [ ] 1.6 Добавить в `bsl-intellisense-v2` resource budget requirement для интерактивного completion.
- [ ] 1.7 Добавить в `bsl-intellisense-v2` low-cardinality resource observability requirement.
- [ ] 1.8 Добавить в `dev-workflow` и `bsl-intellisense-v2` dual latency gate requirement (relative ratio + absolute ceiling).
- [ ] 1.9 Зафиксировать `Option B` как единственный путь: dedicated perf-gate module + versioned schema contract.
- [ ] 1.10 Добавить в `dev-workflow` требование детерминированной `change_criticality` классификации с fail-closed policy.
- [ ] 1.11 Добавить в `dev-workflow` machine-readable test-first evidence contract requirement.
- [ ] 1.12 Добавить в `dev-workflow` bootstrap policy requirement для initial perf budgets.
- [ ] 1.13 Ужесточить `bsl-intellisense-v2` до canonical metric keys без "или эквивалент".

## 2. Design And Tooling
- [ ] 2.1 Утвердить ADR template и критерии "architecturally significant/perf-critical change".
- [ ] 2.2 Определить protected-assets manifest (какие тесты/контракты/baselines считаются immutable в implementation change).
- [ ] 2.3 Спроектировать schema contract v1 (`input/baseline/report`) и format/version policy для resource baseline artifacts.
- [ ] 2.4 Зафиксировать ownership: кто утверждает ADR, кто владелец perf budgets, кто владелец protected-assets policy.
- [ ] 2.5 Зафиксировать абсолютные latency ceilings (`p95/p99`) по профилям `small/large/churn` и policy их изменения через ADR.
- [ ] 2.6 Спроектировать dedicated perf-gate module boundary (API, integration points, reason-code taxonomy).
- [ ] 2.7 Определить schema для `change_criticality` классификации (enum + reason/rule-id + storage path).
- [ ] 2.8 Определить schema для machine-readable test-first evidence (`failing_ref`, `passing_ref`, `scope`, reason-codes).
- [ ] 2.9 Зафиксировать bootstrap методику initial budgets (sample size, aggregation rule, профили, approval path).

## 3. Implementation Rollout
- [ ] 3.1 Реализовать process-gates (ADR/doc-first/protected-assets) как автоматические проверки в CI/локальном workflow.
- [ ] 3.2 Добавить instrumentation completion hot path для allocation/lock metrics.
- [ ] 3.3 Выделить и внедрить dedicated perf-gate module как единственный evaluator для CI/harness/runtime checks.
- [ ] 3.4 Зафиксировать и подключить `contracts/intellisense-perf-gate/v1/**` в pipeline с compatibility-diff проверкой.
- [ ] 3.5 Реализовать extended perf gate в модуле: dual latency gate (relative ratio + absolute ceiling) + resource budgets + fail-closed deterministic report с reason-codes.
- [ ] 3.6 Включить blocking-mode для unified Option B gate после фиксации baseline.
- [ ] 3.7 Реализовать gate-проверки на обязательные canonical metric keys в contract input/report.
- [ ] 3.8 Реализовать fail-closed проверку отсутствия `change_criticality` и отсутствия test-first evidence для соответствующих change-классов.

## 4. Validation
- [ ] 4.1 `openspec validate add-performance-first-ai-engineering-guardrails --strict --no-interactive`.
- [ ] 4.2 Dry-run на репрезентативных профилях (`small`, `large`, `churn`) и подтверждение воспроизводимости отчётов.
- [ ] 4.3 Review с владельцами `analysis-v2`, `runtime`, `LSP`, и process ownership.
- [ ] 4.4 Подтвердить, что в `lsp_server`/скриптах нет альтернативной inline логики perf-verdict вне dedicated module.
- [ ] 4.5 Подтвердить, что bootstrap budgets зафиксированы в versioned contract до включения blocking mode.

## Dependencies / Parallelism
- [ ] D1 Пункты 2.1 и 2.2 блокируют 3.1.
- [ ] D2 Пункты 2.3 и 2.6 блокируют 3.3 и 3.5.
- [ ] D3 Пункт 3.2 может выполняться параллельно с 3.1 после завершения 2.4.
- [ ] D4 Пункт 3.4 блокирует 3.6.
