## 1. Specification
- [x] 1.1 Добавить в `dev-workflow` requirement: compatibility-diff gate для versioned contracts обязателен как manual проверка.
- [x] 1.2 Добавить в `dev-workflow` requirement: breaking diff без major bump должен завершаться fail.
- [x] 1.3 Добавить в `dev-workflow` requirement: при major bump обязателен migration note и отчёт с причиной.

## 2. Design
- [x] 2.1 Зафиксировать формальную классификацию `breaking`/`non_breaking` для contract payload diff.
- [x] 2.2 Зафиксировать формат machine-readable отчёта (pass/fail, violations, compared_versions).
- [x] 2.3 Зафиксировать manual-only rollout policy (`workflow_dispatch`) и критерии перехода к более строгому режиму.

## 3. Implementation (follow-up)
- [x] 3.1 Реализовать checker `scripts/check-contract-compatibility-diff.py` (или эквивалент) с baseline→candidate сравнением.
- [x] 3.2 Интегрировать checker в manual CI job (`workflow_dispatch`) без автозапуска на push/PR.
- [x] 3.3 Добавить regression fixtures/tests для как минимум 2 breaking и 2 non-breaking сценариев.
- [x] 3.4 Документировать запуск и чтение отчёта для review.

## 4. Validation
- [x] 4.1 `openspec validate add-versioned-contract-compatibility-diff-gate --strict --no-interactive`.
- [x] 4.2 Приложить sample compatibility report в `openspec/changes/add-versioned-contract-compatibility-diff-gate/validation/`.
- [x] 4.3 Провести архитектурный review policy с владельцами backend/runtime/extension.
