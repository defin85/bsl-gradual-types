## ADDED Requirements
### Requirement: Superseded completion в `turn_wait` не становится orphaned до active registration (MUST)
Если same-file completion request уже вышел из per-file queue и вошёл в dispatcher `turn_wait`, но ещё не был зарегистрирован как active interactive completion, система MUST продолжать считать его частью same-file latest-wins/cancel lifecycle.

Для такого request система MUST:
- сохранять возможность bounded supersession/cancel до active registration;
- не требовать, чтобы stale request сначала стал active, чтобы затем его можно было остановить;
- не допускать seconds-scale inflight retention stale `turn_wait` request после того, как newer same-file completion или explicit cancel уже сделали его неактуальным;
- не превращать stranded `turn_wait` request в причину seconds-scale `service_future_created -> first poll` backlog для более нового same-file completion.

#### Scenario: Более новый same-file completion вытесняет older request, уже попавший в `turn_wait`
- **GIVEN** request `A` для одного `file_id` уже вышел из per-file queue и ожидает dispatcher turn
- **AND** request `A` ещё не зарегистрирован как active completion owner
- **AND** приходит более новый same-file completion request `B`
- **WHEN** сервер применяет latest-wins semantics
- **THEN** request `A` boundedly получает superseded/cancelled outcome без обязательного перехода в active state
- **AND** request `B` не накапливает seconds-scale pre-poll backlog из-за orphaned `turn_wait` request `A`

#### Scenario: Explicit cancel резолвит `turn_wait` request до active registration
- **GIVEN** completion request уже ожидает dispatcher turn
- **AND** клиент отправил `$/cancelRequest` для этого completion
- **WHEN** adapter и orchestrator обрабатывают cancel
- **THEN** stale request boundedly сворачивается ещё в `turn_wait` lifecycle
- **AND** request не публикует поздний user-facing completion ответ

### Requirement: Completion timeline truthfully отражает `turn_wait` lifecycle текущего request и stale contenders (MUST)
Если authoritative completion timeline публикует absolute `turn_wait` lifecycle текущего request, payload MUST позволять отделить:
- фактическое ожидание current request в `turn_wait`;
- stale contenders, которые всё ещё видимы в `phase=turn_wait`;
- immediate resolve current request без invented multi-second wait.

Сервер MUST NOT схлопывать multi-second current-request `turn_wait` stage в нулевую absolute lifecycle, если такой wait реально наблюдался.
Если current request резолвится immediately, но stale contender остаётся в `phase=turn_wait`, payload MUST показывать это как отдельный contender-state, а не как длительный current-request wait.

#### Scenario: Текущий request проходит `turn_wait` сразу, а stale contender остаётся в `phase=turn_wait`
- **GIVEN** current completion request получает dispatcher-ready outcome практически сразу
- **AND** authoritative contenders всё ещё содержат older same-file completion в `phase=turn_wait`
- **WHEN** оператор читает completion timeline
- **THEN** current-request `turn_wait` absolute lifecycle остаётся immediate
- **AND** stale `turn_wait` contender показывается отдельно через bounded contender fields
- **AND** payload не приписывает multi-second current-request wait только по возрасту stale contender

#### Scenario: Multi-second current `turn_wait` не схлопывается в нулевую absolute lifecycle
- **GIVEN** текущий completion request реально провёл multi-second время в `turn_wait`
- **WHEN** сервер сериализует authoritative completion timeline
- **THEN** absolute `turn_wait` lifecycle остаётся согласованным со stage duration в пределах bounded measurement tolerance
- **AND** payload не выдумывает immediate resolve/wake, если wait реально длился секунды

### Requirement: Same-file overlap gate ловит stranded pre-active `turn_wait` request (MUST)
Acceptance для completion overlap MUST включать сценарий, где older same-file completion теряет актуальность, пока он уже вышел из queue, но ещё не стал active owner.

Этот gate MUST:
- воспроизводить same-file overlap через live LSP path;
- fail-ить, если stale contender остаётся видимым в `phase=turn_wait` за пределами bounded supersession window;
- fail-ить, если новый same-file completion копит seconds-scale `service_future_created -> first poll` backlog из-за stranded pre-active predecessor;
- сохранять checked-in evidence, достаточную для различения pre-active `turn_wait` blind spot от stale active `response_build` retention.

#### Scenario: Representative overlap gate ловит stranded pre-active predecessor
- **GIVEN** live overlap profile на representative real module
- **AND** request `A` уже успел войти в `turn_wait`, но ещё не стал active owner
- **AND** request `B` для того же файла приходит после `A`
- **WHEN** gate измеряет same-file completion overlap
- **THEN** gate требует bounded terminal outcome для `A`
- **AND** gate требует, чтобы `B` достигал first poll без seconds-scale pre-poll backlog из-за stale `turn_wait` predecessor
