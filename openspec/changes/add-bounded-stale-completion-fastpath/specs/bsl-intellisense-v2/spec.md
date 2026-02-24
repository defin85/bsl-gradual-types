## ADDED Requirements

### Requirement: Completion under large-module churn использует dual-path bounded fastpath (MUST)
Для интерактивного completion на больших модулях в состоянии churn система MUST применять двухфазную стратегию:
- latest-path с ограниченным wait budget;
- stale-compatible fallback сразу после исчерпания budget при соблюдении freshness ограничений.

Completion under churn MUST NOT блокироваться секундными хвостами ожидания latest-path, если допустимый stale snapshot доступен.

#### Scenario: Under churn completion отдаёт bounded stale instead of long wait
- **GIVEN** большой модуль находится в активном churn режиме
- **AND** latest-path completion не успевает в wait budget
- **AND** есть stale snapshot, удовлетворяющий freshness ограничениям
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает stale-compatible completion в bounded времени
- **AND** ответ помечается как частичный (`isIncomplete=true`)

### Requirement: После stale serve система выполняет асинхронный latest refresh без user-facing блокировки (MUST)
После выдачи stale completion система MUST запускать background refresh latest snapshot.

Если stale snapshot недоступен или невалиден, completion MUST завершаться быстро в пределах bounded policy без длительного блокирования.

#### Scenario: Stale serve запускает background refresh
- **GIVEN** completion был обслужен через stale fallback
- **WHEN** пользователь продолжает работу
- **THEN** latest refresh выполняется асинхронно в фоне
- **AND** последующие completion запросы могут перейти на latest без блокирующего ожидания предыдущего refresh

### Requirement: Quality gate оценивает churn-aware completion отдельно от non-churn baseline (MUST)
Scale-aware gate MUST публиковать отдельные pass/fail оценки для churn-aware профиля и non-churn baseline.

Gate MUST включать как минимум:
- latency метрики (`completion_duration_ms`, stage-level breakdown);
- stale/fallback counters (`stale_served`, `fallback_unavailable`, `wait_budget_exhausted`);
- sample sufficiency для warm фазы.

#### Scenario: Churn regression выявляется независимо от non-churn профиля
- **GIVEN** non-churn профиль проходит по latency
- **AND** churn-aware профиль деградирует
- **WHEN** выполняется scale-aware gate
- **THEN** gate явно помечает провал churn-aware части
- **AND** отчет содержит stage-level root-cause данные для churn профиля
