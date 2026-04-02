## ADDED Requirements

### Requirement: Existing completion surfaces переносят `v12` first-poll contention attribution без guessed blocker claims (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v12` bounded `first_poll_contention_attribution` в человекочитаемом виде.

Human-readable projection MUST:
- показывать `first_poll_contention_attribution` рядом с existing `v11` first-poll / first-wake split;
- называть этот signal server-visible contender fact, а не "точным виновником";
- явно деградировать на `v11`, не выдумывая `first_poll_contention_attribution`;
- не подменять missing `v12` server attribution client-side probes, correlation heuristics или free-text summary.

#### Scenario: Panel и clipboard показывают видимый contender class рядом с existing ingress facts
- **GIVEN** extension получает completion timeline `v12` с bounded `first_poll_contention_attribution`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible traces
- **THEN** human-readable output показывает новый contender fact рядом с existing dispatch, pre-service-scope, first-poll / first-wake и pre-method facts
- **AND** оператор может увидеть server-visible contender class без открытия raw JSON

#### Scenario: Incident bundle summary переносит `v12` contention facts без overclaim
- **GIVEN** incident bundle строится по `v12` payload
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary сохраняет bounded raw facts из `first_poll_contention_attribution`
- **AND** derived handoff не переименовывает contender class в точный blocking request, request id или URI

#### Scenario: Extension явно деградирует на `v11`
- **GIVEN** connected server возвращает completion timeline `v11`
- **WHEN** extension формирует panel, clipboard или incident bundle
- **THEN** extension не выдумывает `first_poll_contention_attribution`
- **AND** человекочитаемый output явно отмечает, что bounded contender attribution unavailable by design for `contract=v11`
