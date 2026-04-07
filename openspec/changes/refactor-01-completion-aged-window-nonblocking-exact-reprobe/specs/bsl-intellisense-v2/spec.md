## ADDED Requirements

### Requirement: Aged non-member current-revision completion does not block first response on exact re-probe (MUST)

Система MUST формировать aged non-member current-revision first response без blocking exact re-probe,
если exact не был уже доказан из prepared current-revision state.

Если non-member completion request уже использует `shadow_current_revision_fast_path`, current-revision
shadow/support state для requested revision уже подготовлен, а request вышел из immediate apply-age
window, first response MUST NOT синхронно re-probe-ить свежий current-revision snapshot только ради
повторной проверки exact readiness перед terminal decision.

В этом режиме request path:

- MAY возвращать exact только если exact readiness уже доказана из подготовленного current-revision state;
- MUST иначе переходить в bounded lightweight/no-IR current-revision path;
- MUST NOT возвращаться к effectively exact-only first-response поведению;
- MUST NOT получать seconds-scale stall или fail-closed `exact_deadline` только потому, что был сделан post-window exact re-probe.

#### Scenario: Aged non-member invoked completion уходит в bounded current-revision fallback без blocking exact re-probe

- **GIVEN** same-file invoked completion идёт через `shadow_current_revision_fast_path`
- **AND** request не является member-access
- **AND** current-revision shadow/support state для requested revision уже prepared
- **AND** request уже вышел из immediate apply-age window
- **AND** exact readiness не доказана из подготовленного состояния
- **WHEN** handler формирует first response
- **THEN** request не делает blocking exact re-probe как prereq terminal decision
- **AND** возвращает bounded truthful current-revision lightweight/no-IR response
- **AND** не регрессирует в `exact_deadline` только из-за post-window re-probe

### Requirement: Completion timeline truthfully covers blocking current-revision snapshot reacquisition (MUST)

Система MUST truthfully покрывать blocking current-revision snapshot reacquisition в authoritative
completion timeline.

Если completion request path всё ещё делает blocking current-revision snapshot reacquisition или
эквивалентный exact re-probe до terminal first-response decision, authoritative timeline MUST либо:

- явно публиковать эту работу как отдельный low-cardinality stage внутри `stages`, либо
- удерживать разницу между `total_duration_ms` и последним видимым stage end в пределах bounded capture overhead.

Authoritative trace MUST NOT приписывать доминирующую latency unrelated visible stage, если основная
часть request-path времени ушла в неатрибутированную blocking snapshot reacquisition.

#### Scenario: Blocking current-revision snapshot reacquisition не скрывается внутри uncovered handler gap

- **GIVEN** representative aged completion trace тратит заметное время на current-revision snapshot reacquisition до terminal decision
- **WHEN** сервер сериализует authoritative completion timeline
- **THEN** trace либо показывает dedicated low-cardinality stage для этой blocking work
- **OR** не оставляет seconds-scale gap между `total_duration_ms` и последним видимым stage end
- **AND** operator может отличить эту latency от `handler_prelude` и `query_bundle*`
