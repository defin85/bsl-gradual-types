## Context
Даже при deferred-профилях heavy diagnostics под burst `didChange` может оставаться in-flight после того, как ревизия уже superseded. Это создает бесполезную CPU-нагрузку и ухудшает интерактивный путь completion.

## Goals / Non-Goals
- Goals:
  - Прекращать вычисление устаревших heavy diagnostics как можно раньше.
  - Сохранить strict latest-version publish и monotonic инварианты.
  - Сделать отмену наблюдаемой и диагностируемой.
- Non-Goals:
  - Изменять набор или семантику diagnostics правил.
  - Добавлять hard preemption на уровне runtime thread kill.

## Decisions
- Decision 1: Кооперативная отмена вместо принудительного убийства задач
  - Вводятся cancellation checkpoints между тяжелыми стадиями.
  - На supersede ставится cancel signal; задача обязана завершиться с `superseded/cancelled` disposition до publish.

  Alternatives considered:
  - Оставить только post-factum фильтрацию перед publish.
    - Отклонено: не снимает CPU pressure от устаревших задач.

- Decision 2: Единый ключ supersession
  - Ключ включает `file_id`, `profile`, `generation/version`.
  - Новая ревизия заменяет in-flight контекст и инициирует cancel устаревшего.

- Decision 3: Наблюдаемость отмены
  - Обязательные low-cardinality метрики по причинам cancel.
  - Отдельные причины для `superseded_generation`, `superseded_version`, `client_cancel`.

## Risks / Trade-offs
- Риск: слишком частые cancel/restart циклы при высокой частоте правок.
  - Mitigation: debounce и bounded reschedule policy.
- Риск: неполное покрытие checkpoints оставит "доживающие" тяжелые задачи.
  - Mitigation: audit всех heavy стадий + integration tests на burst.
- Риск: race-condition между cancel и publish.
  - Mitigation: обязательная финальная revision/generation проверка перед publish.

## Migration Plan
1. Включить cancel reason метрики и report-only tracing.
2. Активировать supersession cancel для `DebouncedFull` и `IdleHeavy`.
3. Подтвердить отсутствие stale publish на stress-сценариях.

## Open Questions
- Нужен ли per-profile cooldown, чтобы не thrash-ить heavy tasks при экстремальном churn.
- Следует ли различать cancellation budget для syntax и semantic стадий.
