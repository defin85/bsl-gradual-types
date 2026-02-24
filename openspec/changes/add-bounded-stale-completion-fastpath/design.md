## Context
Текущая интерактивная policy уже допускает stale fallback, но под тяжелым large-module churn latest-path может оставаться дорогим и приводить к p95 в десятках секунд, если stale-serve не включается достаточно рано или недостаточно явно оркестрируется.

## Goals / Non-Goals
- Goals:
  - Гарантировать bounded completion latency under churn.
  - Сохранить полезность completion результата через контролируемый stale fallback.
  - Сохранить eventual freshness через асинхронный refresh.
- Non-Goals:
  - Менять candidate ranking или содержимое completion модели.
  - Ослаблять strict latest-version требования для diagnostics publish.

## Decisions
- Decision 1: Двухфазный completion путь
  - Фаза A: latest-path с ограниченным wait budget.
  - Фаза B: stale fallback при соблюдении freshness constraints.
  - Если stale невалиден — быстрый bounded fail/partial outcome без долгого блокирования.

- Decision 2: Явный refresh after stale serve
  - После выдачи stale completion запускается background догоняющий latest refresh.
  - Refresh не блокирует пользовательский completion ответ.

- Decision 3: Churn-aware quality gate
  - Gate оценивает отдельно large/small и churn режимы.
  - Fail критерии учитывают latency и частоту fallback-отказов.

  Alternatives considered:
  - Всегда ждать latest до конца без fallback.
    - Отклонено: неприемлемый tail latency на больших модулях under churn.

## Risks / Trade-offs
- Риск: слишком агрессивный stale fallback ухудшит актуальность подсказок.
  - Mitigation: жесткие freshness constraints + метрики stale usage.
- Риск: refresh backlog после серии stale serve.
  - Mitigation: bounded refresh queue и supersession cancellation.
- Риск: сложнее интерпретировать пользовательский UX (partial/stale).
  - Mitigation: детерминированная маркировка `isIncomplete` + observability.

## Migration Plan
1. Включить fastpath в report-only режиме (метрики без enforce).
2. Подтвердить improvement на churn профиле (`large_warm`, при необходимости `warm_all`).
3. Включить enforce gate и default-on после стабильного canary периода.

## Open Questions
- Нужен ли отдельный wait budget для large-mode по сравнению с глобальным interactive budget.
- Какой допустимый upper bound stale serve rate в steady-state для production rollout.
