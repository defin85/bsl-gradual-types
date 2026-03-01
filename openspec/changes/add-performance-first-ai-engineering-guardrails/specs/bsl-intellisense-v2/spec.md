## ADDED Requirements

### Requirement: Interactive completion v2 имеет обязательные resource budgets по alloc/lock alongside latency (MUST)
Система MUST расширить quality gate интерактивного completion v2: помимо latency, gate MUST учитывать ресурсные бюджеты для warm-path.

Минимальный набор обязательных resource budget метрик:
- `allocations_per_completion` (или эквивалентный детерминированный счётчик аллокаций);
- `allocated_bytes_per_completion` (или эквивалентный memory pressure индикатор);
- `lock_wait_ms_per_completion` и/или `lock_contention_events_per_completion`.

Бюджеты MUST быть versioned в baseline artifact и проверяться на профилях минимум `small`, `large`, `churn`.

#### Scenario: Latency gate проходит, resource gate блокирует регрессию
- **GIVEN** warm latency completion укладывается в целевой SLO
- **WHEN** сравниваются resource metrics с versioned baseline
- **THEN** gate завершается fail, если lock wait или allocations превышают budget
- **AND** change не считается perf-safe

### Requirement: Completion observability публикует low-cardinality allocator/lock pressure signals (MUST)
Система MUST публиковать low-cardinality observability для root-cause анализа resource regressions completion пути.

Observability контракт MUST включать как минимум:
- отдельные метрики/поля для allocation pressure и lock contention;
- фиксированные low-cardinality reason labels (например, `allocator_pressure`, `lock_wait`, `queue_backpressure`, `other`);
- связь resource signals со stage-level completion latency для причинно-следственного drilldown.

#### Scenario: Root-cause деградации локализуется до resource класса
- **GIVEN** зафиксирован рост интерактивной completion latency на churn нагрузке
- **WHEN** анализируется observability snapshot
- **THEN** система позволяет различить allocation pressure и lock contention как отдельные причины
- **AND** причина сопоставляется со stage-level latency без high-cardinality шума

### Requirement: Warm interactive completion избегает process-global lock bottleneck в steady-state (MUST)
Система MUST гарантировать, что warm-path интерактивного completion в steady-state не зависит от process-global lock как обязательной точки сериализации каждого запроса.

Если fallback путь временно использует глобальную сериализацию, это MUST быть:
- явно ограничено редкими условиями;
- отражено в observability как отдельная причина деградации;
- покрыто планом устранения в рамках approved ADR.

#### Scenario: Burst completion не упирается в глобальный lock
- **GIVEN** серия `didChange` и параллельных completion запросов на разных файлах
- **WHEN** система работает в warm steady-state режиме
- **THEN** запросы не сериализуются через process-global lock на каждом completion
- **AND** наблюдаемость не показывает устойчивый global-lock bottleneck как нормальный путь
