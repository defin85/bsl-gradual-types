## ADDED Requirements

### Requirement: Same-file current-revision apply visibility stays ahead of auxiliary parse churn (MUST)
Система MUST обеспечивать, что после того как `didOpen`, `didChange` или `didSave` уже
зарегистрировал same-file handoff для requested revision `V`, наблюдаемое продвижение
`applied_version >= V` для same-file waiters не задерживается по умолчанию из-за same-file
auxiliary parse work, которая не является самим canonical current-revision handoff.

Этот contract MUST покрывать как минимум:
- interactive current-revision waiters, которые опираются на `wait_for_file_version` или
  semantically equivalent applied-state readiness;
- `didSave` diagnostics heavy follow-up после bounded first publish;
- same-file auxiliary parse/snapshot/context work вроде parse snapshot build, same-version refresh,
  `bsl.getCurrentContext`, `documentSymbol` maintenance, `type_index_precompute` или
  semantically equivalent paths.

Система MAY сохранять bounded writer/runtime architecture, но MUST NOT оставлять newest same-file
waiters в состоянии seconds-scale `apply_lag` / `wait_for_file_version` только потому, что впереди
продолжается auxiliary same-file parse churn.

Если stall все же происходит, operator-facing evidence MUST позволять отличить writer/apply backlog
от downstream semantic/query cost.

#### Scenario: didSave follow-up не ждёт latest applied visibility только из-за same-file auxiliary parse churn
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** `didSave` уже сделал bounded first publish и heavy follow-up ждёт applied visibility для той же revision
- **AND** same-file auxiliary parse work все еще активна для того же файла
- **WHEN** backend продвигает applied-state visibility
- **THEN** heavy follow-up не получает primary delay только из-за этой same-file auxiliary parse work
- **AND** any remaining stall атрибутируется другой bounded причине

#### Scenario: Current-revision completion readiness не остаётся stale из-за same-file auxiliary parse work
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** same-file auxiliary parse work для той же revision все еще выполняется
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness wait не остается stale только потому, что same-file auxiliary parse work ещё не завершилась
- **AND** completion не деградирует в `wait_for_file_version` stall по этой причине как default outcome

### Requirement: Large-module same-version auxiliary parse consumers reuse canonical parse truth (MUST)
Для representative large modules same-version auxiliary parse consumers MUST reuse или coalesce
canonical parse truth, keyed by `(file_id, file_version, text_hash)` или semantically equivalent
identity, вместо того чтобы платить repeated independent cold/full parse по идентичному тексту как
default behavior.

Этот contract MUST покрывать как минимум:
- `build_parse_snapshot_v2` или semantically equivalent version-bound parse snapshot builders;
- same-version save-triggered refresh paths;
- `bsl.getCurrentContext`, когда он читает тот же latest shadow text.

Система MAY выполнить один cold/full parse, если отсутствует previous tree или incremental basis,
но после того как same-version parse truth уже available или in-flight:
- later same-version auxiliary consumers MUST reuse it or coalesce behind it;
- `bsl.getCurrentContext` MUST NOT по умолчанию запускать еще один independent full parse того же текста;
- operator-facing evidence MUST сохранять truthful parse mode/fallback distinction вместо маскировки
  repeated full parse как generic background slowdown.

#### Scenario: Current-context переиспользует in-flight same-version parse truth
- **GIVEN** для большого модуля уже идет same-version parse build для revision `V` и text identity `H`
- **AND** `bsl.getCurrentContext` приходит для того же файла и идентичного latest shadow text
- **WHEN** backend обслуживает оба path
- **THEN** current-context reuse-ит existing same-version parse truth или coalesce-ится behind it
- **AND** не запускает independent full parse identical text как default outcome

#### Scenario: Full-text update не превращает каждый same-version auxiliary consumer в новый cold parse
- **GIVEN** large-module `didChange` для revision `V` изначально упал в full parse path из-за отсутствия incremental basis
- **WHEN** later same-version save refresh или другой auxiliary parse consumer читает тот же text identity
- **THEN** later consumer reuse-ит или coalesce-ит existing same-version parse truth
- **AND** identical text не оплачивается repeated independent full parse по умолчанию

### Requirement: Representative conf_big mixed-load gate separates cold parse regressions from apply backlog (MUST)
Representative real-module acceptance для `conf_big`-class degradation MUST включать same-file
mixed-load profile, который одновременно упражняет:
- `didChange`;
- `didSave`;
- auxiliary parse-only load (`bsl.getCurrentContext` или semantically equivalent path);
- waiter на current-revision visibility (`completion` и/или didSave heavy follow-up).

Этот gate MUST:
- собирать authoritative fields, которые различают parse cold start и apply visibility backlog, как
  минимум parse mode/fallback reason, parse build latency, и applied-state wait fields
  (`apply_changes_queue_wait_ms`, `wait_for_file_version`, `apply_lag`, или semantically equivalent);
- fail-ить, если regression проявляется либо как repeated identical same-version full parse by
  default, либо как seconds-scale applied-version lag при healthy truthful transport seams;
- report-ить parse-cold-start cost и writer/apply backlog как separate failure classes, а не
  схлопывать их в один generic runtime wait bucket.

#### Scenario: Representative gate различает repeated full parse и applied-version backlog
- **GIVEN** representative same-file mixed-load profile на `conf_big`-class fixture
- **WHEN** один sample regression-ит repeated same-version full parse, а другой regression-ит through applied-version lag
- **THEN** gate завершается ошибкой
- **AND** evidence явно различает parse-cold-start failure class и writer/apply backlog failure class
