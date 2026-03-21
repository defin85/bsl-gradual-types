## ADDED Requirements
### Requirement: Human-readable completion ingress verdicts остаются truthful и positive-only (MUST)
Derived verdicts для `Completion Timeline` panel, clipboard и связанных extension projections MUST строиться только из уже имеющихся bounded latency fields и MUST NOT маркировать trace как ingress-bottleneck, если соответствующая ingress задержка отсутствует.

Derived verdict layer MUST:
- использовать только существующие bounded waits (`transport_to_method_wait_ms`, `method_prelude_exec_ms` и, при наличии deterministic correlation в downstream consumer, `client_to_transport_wait_ms`);
- строить ingress verdict только при положительной доминирующей задержке;
- различать как минимум `server_before_method_entry_dominant` и `handler_prelude_dominant`;
- MAY различать `client_before_transport_dominant`, если downstream projection уже имеет deterministic probe correlation;
- не выводить generic ingress verdict только потому, что `0 >= 0` или потому что одна из задержек отсутствует.

#### Scenario: Hot trace без положительного ingress wait не получает ingress verdict
- **GIVEN** completion trace имеет `transport_to_method_wait_ms=0` и `method_prelude_exec_ms=0`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает ingress verdict
- **AND** trace не маркируется как `handler_prelude_dominant`

#### Scenario: Server-side wait до method entry доминирует над prelude
- **GIVEN** completion trace имеет положительный `transport_to_method_wait_ms`, который доминирует над `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `server_before_method_entry_dominant`
- **AND** trace не получает `handler_prelude_dominant`

#### Scenario: Handler prelude доминирует над wait до method entry
- **GIVEN** completion trace имеет положительный `method_prelude_exec_ms`, который доминирует над `transport_to_method_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `handler_prelude_dominant`
- **AND** trace не получает server-side ingress verdict

### Requirement: Client-side ingress supplement остаётся fail-closed и deterministic (MUST)
Если extension-projection добавляет human-readable client-side ingress verdict поверх authoritative completion trace, такой verdict MUST появляться только при deterministic probe correlation и положительном доминирующем `client_to_transport_wait_ms`.

Проекция MUST:
- не создавать client-side ingress verdict для uncorrelated или ambiguous requests;
- не использовать probe-only эвристики как substitute для authoritative server verdicts;
- сохранять trace валидным и server-centric, если client correlation недоступна.

#### Scenario: Correlated trace получает client-side ingress verdict
- **GIVEN** request summary имеет deterministic correlation и положительный `client_to_transport_wait_ms`, доминирующий над server-side ingress waits
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `client_before_transport_dominant`
- **AND** server-side verdicts остаются отдельными и не подменяются client-side supplement

#### Scenario: Uncorrelated trace не получает client-side ingress verdict
- **GIVEN** request summary не имеет deterministic probe correlation
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает verdict `client_before_transport_dominant`
- **AND** projection остаётся fail-closed без guessed client-side attribution
