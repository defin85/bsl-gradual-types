## ADDED Requirements

### Requirement: Архитектурно-значимые и perf-critical изменения проходят ADR gate до реализации (MUST)
Для изменений, затрагивающих архитектурно-значимые решения (минимум: модель владения ресурсами, синхронизационные примитивы hot path, модель очередей/cancellation, IPC границы, cache topology), система MUST требовать утвержденный ADR до начала имплементации.

ADR MUST содержать:
- контекст и целевой ASR (latency/memory/contention/correctness);
- минимум две альтернативы и причины выбора;
- ожидаемые бюджеты и критерии успеха;
- rollback/supersede стратегию.

#### Scenario: Perf-critical change без принятого ADR блокируется
- **GIVEN** change затрагивает интерактивный completion hot path и меняет synchronization strategy
- **WHEN** запускается change-review gate
- **THEN** отсутствие принятого ADR приводит к fail
- **AND** implementation этап не считается разрешённым

### Requirement: Non-MVP perf changes выполняются по doc-first контракту (MUST)
Для non-MVP изменений с архитектурным и/или производительным эффектом система MUST требовать полный doc-first пакет до реализации:
- `proposal.md`
- `design.md`
- `tasks.md`
- spec deltas
- acceptance matrix с функциональными и perf проверками.

#### Scenario: Proposal без acceptance matrix не проходит в implementation
- **GIVEN** change помечен как non-MVP и perf-affecting
- **WHEN** выполняется pre-implementation проверка
- **THEN** gate завершается fail, если отсутствует acceptance matrix с критериями pass/fail

### Requirement: Backend/runtime behavioral changes выполняются через test-first цикл (MUST)
Изменения поведения backend/runtime MUST реализовываться через test-first цикл:
- сначала воспроизводимый failing test/contract baseline;
- затем реализация;
- затем минимальный refactor без изменения смысловых acceptance условий.

Система MUST рассматривать отсутствие test-first evidence как нарушение process gate.

#### Scenario: Реализация без воспроизводимого failing test отклоняется
- **GIVEN** PR меняет поведение runtime анализа
- **WHEN** проверяется trace change-to-test
- **THEN** gate завершается fail, если нет зафиксированного failing test/contract baseline до фикса

### Requirement: Protected acceptance assets immutable в implementation change (MUST)
Система MUST защищать protected acceptance assets (ключевые acceptance tests, versioned contracts, perf baselines) от ad-hoc изменений в рамках implementation change.

Если изменение protected assets действительно необходимо, оно MUST выполняться отдельным согласованным change с явной мотивацией и migration note.

#### Scenario: Подгонка тестов под реализацию блокируется
- **GIVEN** implementation change модифицирует protected acceptance tests без отдельного approved change
- **WHEN** запускается protected-assets gate
- **THEN** gate завершается fail с причиной `protected_acceptance_asset_modified`
- **AND** merge блокируется до согласованного test/contract update path

### Requirement: Perf-critical merge gate требует resource evidence, а не только latency (MUST)
Для perf-critical изменений система MUST требовать детерминированные before/after артефакты с минимумом метрик:
- latency (`p50/p95/p99` для целевого interactive пути);
- allocations (количество и/или bytes per operation);
- lock contention / lock wait.

Gate MUST падать при отсутствии обязательных артефактов или выходе за утверждённые бюджеты.

#### Scenario: Latency улучшилась, но allocation budget нарушен
- **GIVEN** change показывает лучшее latency в warm профиле
- **WHEN** perf merge gate анализирует resource evidence
- **THEN** gate завершается fail, если allocations выходят за budget
- **AND** change не принимается до корректировки реализации или явного обновления budget через ADR
