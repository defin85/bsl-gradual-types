## ADDED Requirements

### Requirement: Same-file didChange current-revision handoff registers ahead of full handler work (MUST)
Система MUST регистрировать current-revision `SetFile` handoff и публиковать same-file ingress
token для `(file_id, V)` через минимальный ingress fast lane после того, как same-file
`textDocument/didChange` для requested revision `V` уже принят и декодирован и сервер может
вычислить canonical updated text для этого change, но раньше, чем delayed full-handler work
(`lsp_did_change`, parse-snapshot scheduling, diagnostics scheduling или другой same-file
auxiliary work) сможет seconds-scale удерживать later completion для того же файла.

Этот fast lane MUST:

- обновлять `latest_received` и same-file shadow state именно тем текстом, который принят для
  `didChange` revision `V`;
- публиковать same-file ingress token только после того, как current-revision handoff для
  `(file_id, V)` действительно зарегистрирован;
- сохранять latest-wins и out-of-order semantics для same-file revisions;
- не допускать, чтобы downstream handler path double-apply-ил тот же `SetFile` или публиковал
  более сильную readiness semantics, чем реально был зарегистрированный handoff.

#### Scenario: Later completion no longer waits for full didChange handler entry
- **GIVEN** same-file `didChange` для revision `V` уже достиг server ingress
- **AND** сервер уже может вычислить canonical updated text для этого change
- **AND** full `lsp_did_change` handler work для той же notification ещё не завершилось
- **WHEN** позже приходит completion request для того же файла
- **THEN** completion MAY ждать truthful current-revision handoff для revision `V`
- **AND** completion MUST NOT spend seconds-scale same-file wait только потому, что
  `didChange` ещё не достиг full handler entry или его later auxiliary stages

#### Scenario: Dispatcher bookkeeping alone does not publish same-file freshness
- **GIVEN** same-file `didChange` для revision `V` уже создал barrier-owner или другое transport
  bookkeeping
- **AND** current-revision handoff для `(file_id, V)` ещё не зарегистрирован
- **WHEN** оператор читает authoritative completion trace
- **THEN** same-file ingress token для revision `V` остаётся не опубликован
- **AND** система не считает later same-file completion wait-free только по факту раннего
  dispatcher bookkeeping

#### Scenario: Superseded older didChange cannot re-publish stale same-file token
- **GIVEN** same-file `didChange` для revision `V` уже in-flight на fast lane
- **AND** затем приходит более новая revision `V+1` для того же файла
- **WHEN** latest-wins semantics выбирают текущую authoritative revision
- **THEN** older revision `V` MUST NOT publish or re-publish a same-file ingress token, который
  может задержать или исказить completion для `V+1`
- **AND** later same-file completion ждёт только ту revision, которая остаётся authoritative

### Requirement: Representative mixed-load evidence fails on post-didChange handoff lag (MUST)
Representative same-file mixed-load validation для крупного модуля MUST завершаться ошибкой, если
later completion всё ещё проводит seconds-scale время в `completion_barrier_wait_ms` или
`same_file_ingress_token_wait_ms` после того, как earlier same-file `didChange` уже наблюдался на
server ingress для требуемой revision и positive client/output-side waits не объясняют outlier.

Checked-in evidence для этого gate MUST сохранять хотя бы один correlation slice, который
показывает:

- requested revision completion trace;
- barrier owner revision, если owner присутствует;
- когда same-file handoff/token стал observable для этой revision.

#### Scenario: Live gate fails when handoff publication still lags after didChange ingress
- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** same-file `didChange` для requested revision уже наблюдался на server ingress до later
  completion trace
- **WHEN** measured completion sample всё ещё тратит seconds-scale время в
  `completion_barrier_wait_ms` или `same_file_ingress_token_wait_ms`
- **THEN** representative gate завершается ошибкой
- **AND** regression не маскируется под generic client ingress, output handoff или cold
  query-body latency

#### Scenario: Worst outlier evidence preserves same-file revision ownership
- **GIVEN** representative same-file mixed-load profile уже поймал worst completion outlier
- **WHEN** оператор читает checked-in evidence
- **THEN** evidence сохраняет correlation slice c requested revision и barrier owner revision,
  когда owner доступен
- **AND** по evidence можно понять, когда same-file handoff/token стал observable для этой
  completion path
