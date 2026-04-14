## ADDED Requirements

### Requirement: Exact ready-snapshot producer экспортирует phase-level latency attribution (MUST)

Система MUST экспортировать bounded phase-level latency attribution для exact ready-snapshot
producer на пути от начала blocking parse до момента, когда ready snapshot уже установлен и
queryable для exact target revision.

Этот attribution MUST различать как минимум:

- `parse_exec`;
- `post_parse_pre_materialization`;
- `ready_install`.

Работа после ready install, включая documentSymbol / outline side-work, MUST экспортироваться
отдельно и MUST NOT искусственно увеличивать exact readiness phase.

#### Scenario: Bundle показывает, что timeout произошёл во время parse phase

- **GIVEN** `didSave` bounded wait ждёт exact still-current ready-snapshot producer
- **AND** budget истекает до materialization
- **WHEN** оператор экспортирует incident bundle
- **THEN** bundle показывает producer phase at timeout
- **AND** если exact worker ещё находился в blocking parse, dominant phase указывает на
  `parse_exec`

#### Scenario: Symbol side-work не маскируется под exact readiness

- **GIVEN** ready snapshot уже установлен для exact target revision
- **AND** после этого ещё выполняется documentSymbol / outline side-work
- **WHEN** observability payload summarises ready-snapshot lifecycle
- **THEN** exact readiness phase заканчивается на ready install
- **AND** symbol/outline side-work показывается как отдельная non-readiness phase
