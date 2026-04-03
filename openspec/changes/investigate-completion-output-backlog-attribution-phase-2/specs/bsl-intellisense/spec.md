## MODIFIED Requirements

### Requirement: Existing completion surfaces переносят `v23` truthful output backlog attribution без guessed culprit (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v23` backlog attribution в человекочитаемом виде.

Human-readable projection MUST:

- показывать `output_messages_ahead_count`, `output_bytes_ahead_estimate` и `output_head_blocker_class`, если connected server возвращает `v23` payload с truthful backlog snapshot;
- сохранять `response_output_queue_wait_ms` как primary timing bucket и использовать backlog snapshot как supporting evidence;
- явно деградировать на `v22`, не выдумывая backlog snapshot или blocker class;
- не трактовать `output_bytes_ahead_estimate` как exact flushed byte count.

#### Scenario: Panel и clipboard показывают truthful backlog attribution

- **GIVEN** extension получает completion timeline `v23`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** output показывает queue wait вместе с bounded ahead snapshot
- **AND** оператору не нужно читать raw JSON, чтобы увидеть coarse backlog culprit

#### Scenario: Incident bundle summary явно деградирует на `v22`

- **GIVEN** connected server возвращает `v22` payload без truthful backlog snapshot
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary явно отмечает, что truthful backlog attribution unavailable by design для этой evidence version
- **AND** derived handoff не называет `response_output_queue_wait_ms` конкретным notification/executeCommand/other_request culprit без нового authoritative snapshot
