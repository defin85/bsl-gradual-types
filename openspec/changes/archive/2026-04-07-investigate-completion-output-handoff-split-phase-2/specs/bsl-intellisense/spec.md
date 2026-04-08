## MODIFIED Requirements

### Requirement: Existing completion surfaces переносят `v24` post-handler handoff split без guessed culprit (MUST)
Completion Timeline panel, clipboard export и request-centric incident bundle summary MUST переносить `v24` post-handler handoff split в человекочитаемом виде.

Human-readable projection MUST:

- показывать `response_ready_to_output_handoff_wait_ms`, `response_output_handoff_send_wait_ms` и `response_output_handoff_to_writer_wait_ms`, если connected server возвращает `v24` payload с truthful `response_output_handoff_*` boundaries;
- сохранять `response_ready_to_output_enqueue_wait_ms` как compatibility umbrella и MAY показывать его рядом с finer `v24` split;
- явно помечать `response_output_enqueue_completed_at_ms` как legacy compatibility seam и не называть её truthful enqueue completion;
- явно деградировать на `v23`, не выдумывая finer handoff boundaries;
- не переименовывать `response_output_handoff_send_wait_ms` или `response_output_handoff_to_writer_wait_ms` в writer backlog, notification culprit, executeCommand blocker или иной более точный виновник без дополнительных authoritative fields.

#### Scenario: Panel и clipboard показывают truthful post-handler handoff split

- **GIVEN** extension получает completion timeline `v24`
- **WHEN** оператор открывает Completion Timeline panel или копирует visible trace
- **THEN** output показывает delay до handoff start, send-side handoff wait и wait до writer selection отдельно
- **AND** оператору не нужно читать raw JSON, чтобы увидеть, где именно теряется post-handler время до legacy writer-selection seam

#### Scenario: Incident bundle summary явно деградирует на `v23`

- **GIVEN** connected server возвращает `v23` payload без truthful handoff boundaries
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary явно отмечает, что truthful pre-enqueue handoff split unavailable by design для этой evidence version
- **AND** derived explanation не переименовывает opaque `response_ready_to_output_enqueue_wait_ms` в точный writer-backlog или иной culprit
