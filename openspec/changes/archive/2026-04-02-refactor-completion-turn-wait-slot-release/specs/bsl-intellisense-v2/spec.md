## ADDED Requirements
### Requirement: Event-driven completion освобождает transport slot до длительного passive `turn_wait` (MUST)
На default event-driven completion path LSP request MUST NOT удерживать `tower-lsp` transport admission slot только потому, что request пассивно ждёт dispatcher turn или older same-file turn owner.

Перед таким wait система MUST:
- захватить request correlation и cancellation context, необходимые для normal completion response path;
- зафиксировать completion-owned handoff boundary, после которой passive wait больше не считается transport-slot retention;
- сохранить same-file latest-wins/cancel semantics для request, который ещё не начал heavy completion work.

#### Scenario: Current same-file completion ждёт older owner без seconds-scale pre-first-poll backlog
- **GIVEN** completion request `B` для файла приходит, пока older same-file request `A` ещё удерживает dispatcher turn
- **AND** `B` должен подождать release текущего owner, прежде чем начать heavy completion stages
- **WHEN** сервер принимает `B` на default event-driven path
- **THEN** transport admission slot освобождается до multi-second passive `turn_wait`
- **AND** authoritative trace не показывает seconds-scale `service_future_created -> first_poll` backlog только из-за ожидания turn для `B`
- **AND** `B` позже продолжает completion lifecycle по normal response path

#### Scenario: Explicit cancel останавливает completion после handoff, но до heavy work
- **GIVEN** completion request уже прошёл handoff boundary и ещё только пассивно ждёт dispatcher turn
- **AND** клиент отправляет `$/cancelRequest` для этого completion
- **WHEN** adapter и completion orchestrator обрабатывают cancel
- **THEN** request boundedly сворачивается без late publish user-facing completion ответа
- **AND** transport slot не удерживается до терминального завершения этого passive wait

### Requirement: Post-handoff completion сохраняет single-owner и exactly-once terminal semantics (MUST)
После completion handoff система MUST назначать ровно одного lifecycle owner, который владеет:
- `request_id` и correlation context для terminal response path;
- cancellation/shutdown cleanup;
- правом отправить не более одного terminal response или завершить request fail-closed, если transport уже недоступен.

Dispatcher MUST оставаться единственным authority для `latest-wins` и publishability. Post-handoff completion task MUST NOT самостоятельно становиться publishable в обход dispatcher/epoch checks.

#### Scenario: Cancel race не приводит к двойному terminal response
- **GIVEN** completion request уже передан post-handoff owner и ещё не начал heavy work
- **AND** почти одновременно приходят `$/cancelRequest` и wakeup/resolution для ожидания turn
- **WHEN** lifecycle owner и dispatcher обрабатывают эту гонку
- **THEN** для данного `request_id` наблюдается не более одного terminal outcome
- **AND** request не публикует поздний completion ответ после terminal cleanup

#### Scenario: Supersede race сохраняет latest-wins и exactly-once cleanup
- **GIVEN** older same-file completion уже передан post-handoff owner
- **AND** newer same-file completion supersedes older request до начала heavy work
- **WHEN** dispatcher и lifecycle owner обрабатывают supersede
- **THEN** older request получает bounded terminal cleanup ровно один раз
- **AND** newer request остаётся единственным publishable same-file completion

#### Scenario: Shutdown race завершает handoff owner fail-closed без late publish
- **GIVEN** completion request уже передан post-handoff owner
- **AND** server shutdown начинается до terminal completion response
- **WHEN** lifecycle owner обрабатывает shutdown
- **THEN** owner boundedly завершает cleanup без двойного terminal response
- **AND** после shutdown не появляется поздний publish user-facing completion ответа

### Requirement: Completion timeline отделяет off-transport wait от ingress backlog (MUST)
Если authoritative completion timeline публикует latency profile request, payload MUST позволять отделить:
- ingress backlog до handoff / admission;
- completion-owned wait после handoff;
- stale contenders, которые всё ещё видимы в `phase=turn_wait`.

Сервер MUST NOT объяснять multi-second off-transport wait через `service_future_created -> first_poll` или через handler-resident passive `turn_wait`, если transport slot уже освобождён.

#### Scenario: First poll bounded, а multi-second wait идёт после handoff
- **GIVEN** current completion request быстро проходит transport admission path
- **AND** затем request проводит multi-second время в passive wait за same-file turn owner
- **WHEN** оператор читает authoritative completion timeline
- **THEN** payload сохраняет bounded ingress attribution до handoff
- **AND** multi-second completion-owned wait показывается отдельно от `service_future_created -> first_poll`
- **AND** payload не маскирует off-transport wait под transport backlog

#### Scenario: Connected server ещё не поддерживает handoff-aware contract
- **GIVEN** connected server возвращает timeline старой версии без новых handoff-aware полей
- **WHEN** extension или operator читает payload
- **THEN** клиент не выдумывает off-transport wait attribution
- **AND** trustworthy semantics остаются ограничены реально присутствующими полями

### Requirement: Same-file overlap gate ловит completion `turn_wait` transport-slot retention (MUST)
Acceptance для same-file overlap MUST fail-ить не только на stranded contender или pre-first-poll backlog, но и на сценарий, где current completion request всё ещё проводит seconds-scale passive `turn_wait` внутри transport/handler path.

Этот gate MUST:
- воспроизводить same-file overlap через live LSP default path;
- fail-ить, если current request удерживает transport/handler path на multi-second passive `turn_wait`;
- fail-ить, если same-file overlap снова превращает completion lifecycle в ingress bottleneck;
- сохранять checked-in evidence, достаточную для различения ingress backlog, stale contender и off-transport wait.

#### Scenario: Representative overlap gate ловит inline `turn_wait`, удерживающий transport path
- **GIVEN** live same-file overlap profile на representative real module
- **AND** request `B` приходит, пока older same-file request `A` всё ещё удерживает turn
- **WHEN** gate измеряет latency profile `B`
- **THEN** gate требует bounded transport admission path для `B`
- **AND** gate завершает прогон ошибкой, если multi-second passive `turn_wait` всё ещё наблюдается внутри transport/handler path
- **AND** checked-in evidence позволяет отличить этот regression от stale pre-active contender
