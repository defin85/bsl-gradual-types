## ADDED Requirements

### Requirement: `bsl.getCurrentContext` honors client latest-only generations with bounded supersession (MUST)

Server MUST honor bounded client latest-only generations for `bsl.getCurrentContext`.

Если client current-context surface передаёт bounded generation hints для `bsl.getCurrentContext`,
server MUST использовать их для bounded supersession/coalescing obsolete auxiliary work.

Для одного editor session backend:

- MUST NOT позволять obsolete older generations неограниченно накапливать independent expensive parse/context derivation;
- MUST supersede older generation до expensive parse/context derivation или коалесцировать её с эквивалентным newer work;
- MUST NOT делать obsolete response источником current context для newer generation;
- MAY по-прежнему возвращать bounded auxiliary response для superseded request, если это не нарушает newest-generation-wins semantics на client side.

#### Scenario: Cursor burst supersede-ит obsolete current-context work до expensive parse

- **GIVEN** extension отправляет несколько `bsl.getCurrentContext` requests одного editor session с монотонно растущими generation hints
- **AND** более новая generation становится известна серверу до завершения expensive parse для older request
- **WHEN** backend обслуживает этот burst
- **THEN** older request не доходит независимо до полного expensive parse/context derivation
- **AND** auxiliary path остаётся bounded по obsolete work
- **AND** newer generation остаётся единственным current candidate для client-visible context surface
