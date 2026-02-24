## 1. Fastpath Contract
- [ ] 1.1 Зафиксировать churn-aware bounded completion fastpath и его preconditions.
- [ ] 1.2 Определить критерии stale-acceptance (version gap, age, deps/settings compatibility).

## 2. Completion Orchestration
- [ ] 2.1 Реализовать immediate stale fallback после исчерпания latest wait budget без секундного блокирования.
- [ ] 2.2 Добавить background refresh path после stale serve для догоняющего latest snapshot.
- [ ] 2.3 Гарантировать deterministic outcome: latest response предпочтителен, stale используется только при budget exceed.

## 3. Observability
- [ ] 3.1 Расширить метрики stale-served/fallback-unavailable/budget-exhausted для churn сценариев.
- [ ] 3.2 Добавить разрезы по `large/small` и `churn mode` для quality gate отчетов.

## 4. Validation
- [ ] 4.1 Добавить integration тесты: under churn completion остается bounded и не деградирует в transient terminal-empty.
- [ ] 4.2 Обновить scale-aware perf gate (`large/small`, `start/cold/warm`) с отдельной оценкой stale fastpath.
- [ ] 4.3 Выполнить `openspec validate add-bounded-stale-completion-fastpath --strict --no-interactive`.
