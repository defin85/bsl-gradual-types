## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version
`25`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через
`workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской
реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline
`contracts/lsp-completion-timeline/v22`, синхронизированный с текущим authoritative payload и его
bounded field-set.

`v25` MUST сохранять additive `v24` ingress/query-body/flush-aware/output-egress semantics,
включая grouped `query_bundle*` taxonomy, `response_sent_at_ms`, existing `response_output_*`
milestones и `response_flush_completed_at_ms`.

Контракт `v25` MUST включать:

- `version` (числовой номер контракта);
- `traces` (массив completion trace записей).

Каждый trace MUST включать:

- `trace_id`, `request_id`, `uri`, `trigger_mode`;
- `outcome`, `started_at_ms`, `total_duration_ms`;
- `dominant_stage`;
- `prepare_details`;
- `turn_attribution`;
- optional `server_edge_details`;
- `stages`.

Если `server_edge_details` присутствует, additive `v25` pre-dispatch decomposition MAY
включать:

- `adapter_read_started_at_ms`;
- `adapter_parse_completed_at_ms`;
- `read_loop_wait_reason`;
- `read_loop_wait_ms`;
- `pending_completion_spillover_depth`;
- `pending_general_request_staged`;
- `admission_try_enqueue_at_ms`;
- `admission_lane`;
- `admission_lane_depth_before`;
- `admission_lane_depth_after`;
- `admission_enqueue_outcome`;
- `admission_spillover_outcome`;
- `admission_enqueued_at_ms`;
- `admission_queue_wait_ms`;
- `scheduler_woke_at_ms`;
- `scheduler_poll_ready_entered_at_ms`;
- `scheduler_poll_ready_resolved_at_ms`;
- `scheduler_poll_ready_wait_ms`;
- `scheduler_dequeued_at_ms`;
- `completion_barrier_active_at_dequeue`;
- `completion_barrier_generation`;
- `completion_barrier_owner_method`;
- `completion_barrier_owner_uri`;
- `completion_barrier_owner_version`;
- `completion_barrier_wait_ms`;
- `scheduler_service_call_started_at_ms`;
- `scheduler_service_call_returned_at_ms`;
- `scheduler_service_call_sync_exec_ms`;
- `doc_sync_first_poll_exec_ms`;
- `doc_sync_first_poll_outcome`;
- `doc_sync_first_poll_method`;
- `doc_sync_first_poll_uri`;
- `doc_sync_first_poll_version`;
- `same_file_ingress_token_required_version`;
- `same_file_ingress_token_published_at_ms`;
- `same_file_ingress_token_source`;
- `same_file_ingress_token_wait_ms`;
- `scheduler_ready_to_dispatch_wait_ms`.

`read_loop_wait_reason` MUST использовать только bounded vocabulary:

- `completion_lane_space`;
- `general_lane_space`;
- `none`.

Если `read_loop_wait_reason` присутствует и не равен `none`, payload MUST включать и
`read_loop_wait_ms`.

Если `pending_completion_spillover_depth` присутствует, payload MUST отражать queue depth на
момент reader-side wait, а не post-facto агрегированную оценку.

Если `admission_lane_depth_before` или `admission_lane_depth_after` присутствуют, они MUST
описывать depth именно того admission lane, в который пытались enqueue текущий request.

`admission_lane` MUST использовать только bounded vocabulary:

- `control`;
- `interactive_completion`;
- `document_sync_ingress`;
- `general`.

Если `admission_enqueued_at_ms` присутствует, payload MUST включать и
`admission_queue_wait_ms`.

Если `scheduler_poll_ready_resolved_at_ms` присутствует, payload MUST включать и
`scheduler_poll_ready_wait_ms`, и `scheduler_ready_to_dispatch_wait_ms`.

Если `completion_barrier_active_at_dequeue=true`, payload SHOULD публиковать и
`completion_barrier_owner_method`; если owner относится к file-scoped document-sync path, payload
SHOULD публиковать и `completion_barrier_owner_uri`, и `completion_barrier_owner_version`.

Если `same_file_ingress_token_published_at_ms` присутствует, payload MUST включать и
`same_file_ingress_token_required_version`, и `same_file_ingress_token_wait_ms`.

Если `same_file_ingress_token_source` присутствует, payload MUST использовать bounded vocabulary:

- `did_open`;
- `did_change`;
- `did_save`;
- `did_close`;
- `other`.

Если additive `v25` admission split присутствует, compatibility field
`adapter_to_dispatch_wait_ms` MUST сохранять umbrella semantics для полного server-side интервала
между `adapter_read_at_ms` и earliest dispatch boundary.

Если additive `v25` admission split присутствует полностью, сумма
`admission_queue_wait_ms + scheduler_poll_ready_wait_ms + completion_barrier_wait_ms + same_file_ingress_token_wait_ms + scheduler_ready_to_dispatch_wait_ms`
MUST совпадать с `adapter_to_dispatch_wait_ms`.

#### Scenario: `v25` payload раскладывает local reader wait и `adapter_read -> dispatch`

- **GIVEN** completion request сначала столкнулся с reader-side wait из-за local spillover или
  затем уже был задержан до dispatch в service pipeline
- **WHEN** оператор читает `server_edge_details`
- **THEN** payload может публиковать `read_loop_wait_reason`, `read_loop_wait_ms`,
  `pending_completion_spillover_depth`, `admission_lane`, `admission_enqueued_at_ms`,
  `admission_queue_wait_ms`, `scheduler_poll_ready_resolved_at_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_required_version`, `same_file_ingress_token_published_at_ms`,
  `same_file_ingress_token_wait_ms` и `scheduler_ready_to_dispatch_wait_ms`
- **AND** `adapter_to_dispatch_wait_ms` остаётся compatibility umbrella для всего
  `adapter_read -> dispatch` окна

#### Scenario: Versioned contract baseline синхронизирован с shipped payload

- **GIVEN** authoritative completion timeline уже публикует contract `v25`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v22` совпадает по bounded field-set с runtime
  payload
- **AND** policy/verification scripts валидируют именно `v25/v22`, а не более старую версию

### Requirement: Human-readable completion ingress verdicts остаются truthful и positive-only (MUST)
Derived verdicts для `Completion Timeline` panel, clipboard и связанных extension projections MUST
строиться только из уже имеющихся bounded latency fields и MUST NOT маркировать trace как
ingress-bottleneck, если соответствующая ingress задержка отсутствует.

Derived verdict layer MUST:

- использовать bounded waits `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_wait_ms`, `adapter_to_dispatch_wait_ms`,
  `transport_to_method_wait_ms`, `method_prelude_exec_ms` и, при наличии deterministic
  correlation в downstream consumer, `client_to_transport_wait_ms`;
- строить ingress verdict только при положительной доминирующей задержке;
- различать как минимум `reader_backpressure_dominant`, `admission_queue_dominant`,
  `scheduler_poll_ready_dominant`, `completion_barrier_dominant`,
  `same_file_ingress_token_dominant`,
  `adapter_before_dispatch_dominant`, `server_before_method_entry_dominant` и
  `handler_prelude_dominant`;
- использовать `adapter_before_dispatch_dominant` как backward-compatible umbrella verdict только
  если finer `v25` admission split отсутствует;
- MAY различать `client_before_transport_dominant`, только если deterministic correlation уже
  доказала положительный wait до самой ранней authoritative server ingress boundary, local
  `read_loop_wait_ms` отсутствует или не доминирует, и server-side `v25` admission buckets не
  объясняют задержку;
- не выводить generic ingress verdict только потому, что `0 >= 0` или потому что одна из
  задержек отсутствует.

#### Scenario: Reader-side spillover dominates before dispatch

- **GIVEN** completion trace имеет положительный `read_loop_wait_ms`, вызванный
  `read_loop_wait_reason=completion_lane_space`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `reader_backpressure_dominant`
- **AND** trace не получает verdict `client_before_transport_dominant`

#### Scenario: Queue residence доминирует над shared readiness и handler prelude

- **GIVEN** completion trace имеет положительный `admission_queue_wait_ms`, который доминирует
  над `scheduler_poll_ready_wait_ms`, `same_file_ingress_token_wait_ms`,
  `transport_to_method_wait_ms` и `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `admission_queue_dominant`
- **AND** trace не получает verdict `client_before_transport_dominant`

#### Scenario: Shared readiness доминирует над queue residence

- **GIVEN** completion trace имеет положительный `scheduler_poll_ready_wait_ms`, который
  доминирует над `admission_queue_wait_ms`, `transport_to_method_wait_ms` и
  `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `scheduler_poll_ready_dominant`
- **AND** trace не деградирует в coarse `adapter_before_dispatch_dominant`, если `v25`
  admission split уже присутствует

#### Scenario: Completion barrier dominates and the owner stays attributable

- **GIVEN** completion trace имеет положительный `completion_barrier_wait_ms`, который
  доминирует над `admission_queue_wait_ms`, `scheduler_poll_ready_wait_ms` и
  `same_file_ingress_token_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `completion_barrier_dominant`
- **AND** authoritative payload сохраняет barrier owner attribution, если она была доступна на
  server side

#### Scenario: Server-side admission split suppresses false client ingress blame

- **GIVEN** request summary имеет deterministic probe correlation
- **AND** authoritative payload содержит положительный `admission_queue_wait_ms` или
  `scheduler_poll_ready_wait_ms` или `read_loop_wait_ms` или `same_file_ingress_token_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает verdict `client_before_transport_dominant`
- **AND** projection остаётся fail-closed по client-side supplement

### Requirement: Interactive completion admission изолирован от general LSP backlog до dispatch (MUST)
Система MUST изолировать `textDocument/completion` от unrelated general LSP traffic в окне между
чтением request transport adapter'ом и dispatch в service pipeline.

Изоляция MUST обеспечивать:

- shared readiness/admission state MUST принадлежать одному scheduler owner; reader/producers MUST
  NOT вызывать `poll_ready()/call()` напрямую;
- completion request классифицируется и попадает в interactive admission queue до shared
  readiness blocking для general traffic;
- general requests MUST NOT удерживать freshly-read completion request вне interactive admission
  queue только из-за общего readiness wait;
- completion-supporting document-sync notifications
  (`textDocument/didOpen`, `textDocument/didChange`, `textDocument/didSave`,
  `textDocument/didClose`) MUST публиковать same-file ingress ownership/token через
  per-file owner, который применяет raw document ordering для этого файла и делает latest
  handoff observable до того, как later completion для того же файла зависит от него;
- same-file ingress token MUST публиковаться только после регистрации current-revision handoff
  для соответствующего `(file_id, version)`, а не на более ранней dispatcher-event boundary;
- once the relevant same-file ingress token is already published, unrelated same-priority work для
  других файлов MUST NOT удерживать later completion first response только из-за shared FIFO
  residence;
- control traffic (`$/cancelRequest`, shutdown-related flow) MAY preempt queued completion
  admission;
- saturated completion spillover MUST оставаться bounded и fail-closed: older queued completion MAY
  завершаться pre-dispatch outcome `queue_rejected`, но transport runtime MUST NOT деградировать в
  reader stall, который мешает позднему control traffic даже быть классифицированным;
- queued completion cancellation MUST сохранять existing exactly-once terminal semantics, MUST
  возвращать ровно один terminal response и MUST NOT допускать late publish после признанного
  cancel.

#### Scenario: Same-file ingress token делает completion независимым от unrelated same-priority FIFO

- **GIVEN** transport runtime уже держит queued work для других файлов
- **AND** для файла `F` same-file `didChange` или `didSave` уже опубликовал актуальный ingress
  token
- **AND** затем приходит completion request для того же файла `F`
- **WHEN** сервер формирует first response для completion
- **THEN** completion зависит от ingress token файла `F`, а не от unrelated same-priority FIFO
  residence
- **AND** first response не сидит seconds-scale только потому, что раньше были прочитаны
  unrelated document-sync requests для других файлов

#### Scenario: Dispatcher event не считается same-file ingress token publication

- **GIVEN** `didChange` для файла `F` уже был отправлен в completion dispatcher
- **AND** current-revision handoff для `(F, version)` ещё не зарегистрирован
- **WHEN** оператор читает authoritative trace
- **THEN** payload не считает same-file ingress token опубликованным
- **AND** later completion для файла `F` не может считаться wait-free только по факту раннего
  dispatcher event

#### Scenario: Queued completion отменяется до dispatch без late publish

- **GIVEN** completion request уже стоит в pre-dispatch queue
- **AND** до его dispatch приходит matching `$/cancelRequest`
- **WHEN** scheduler обрабатывает control lane
- **THEN** queued completion помечается cancelled до dispatch
- **AND** сервер возвращает ровно один terminal response с cancellation semantics
  `RequestCancelled`
- **AND** authoritative trace публикует outcome `cancelled` без выдуманных post-dispatch
  timestamps

### Requirement: Representative mixed-load guard budgets truthful ingress and handoff seams (MUST)
Representative mixed-load regression coverage для completion MUST budget-ить truthful latency
seams, которые остаются user-visible после `v25` admission decomposition, а не только legacy
pre-dispatch ingress split.

Guard MUST как минимум:

- использовать same-file profile `didChange + didSave + documentSymbol burst + completion` на
  representative large-module fixture;
- собирать authoritative fields `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms`,
  `same_file_ingress_token_wait_ms`, `client_to_transport_wait_ms`,
  `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`;
- fail-ить, если same-file completion after the relevant ingress token is already published всё
  равно получает seconds-scale `read_loop_wait_ms`, `admission_queue_wait_ms`,
  `scheduler_poll_ready_wait_ms`, `completion_barrier_wait_ms` или
  `same_file_ingress_token_wait_ms`;
- fail-ить, если regression снова маскируется как client-side ingress, когда authoritative
  server-side `v25` admission split уже объясняет задержку;
- сохранять existing correctness checks для non-empty completion, fail-closed counters и
  `documentSymbol latest_ready` behavior.

#### Scenario: Representative gate ловит same-file residual after ready ingress token without bucket shift

- **GIVEN** representative same-file mixed-load profile на крупном модуле
- **AND** relevant same-file ingress token уже опубликован до measured completion
- **WHEN** measured completion sample всё ещё проводит seconds-scale время в
  `read_loop_wait_ms`, `admission_queue_wait_ms`, `scheduler_poll_ready_wait_ms`,
  `completion_barrier_wait_ms` или `same_file_ingress_token_wait_ms`
- **THEN** gate завершается ошибкой
- **AND** regression не маскируется под generic client ingress или cold query-body cost

#### Scenario: Representative evidence keeps a correlation slice for the worst outlier

- **GIVEN** representative mixed-load profile на крупном модуле уже поймал worst completion outlier
- **WHEN** оператор читает checked-in evidence
- **THEN** evidence сохраняет хотя бы один correlation slice с active same-file freshness pressure
  when present
- **AND** этот slice может включать barrier owner, required token version, current published token
  version/source и timestamps, достаточные чтобы сопоставить outlier с overlapping didChange train

## ADDED Requirements

### Requirement: Transport runtime progression stays task-isolated from scheduler/service work (MUST)
The system MUST keep transport runtime progression task-isolated from scheduler/service work.

Transport runtime loops, отвечающие за adapter read/decode/classify, single-owner scheduling и
output/handoff progression, MUST выполняться на independently progressing async tasks или на
эквивалентной starvation-safe boundary. Long-running pre-await work, readiness wait или barrier
handling в одном loop MUST NOT по конструкции останавливать остальные loops.

Этот contract MUST гарантировать как минимум:

- поздний adapter read/decode/classify может продолжаться, даже если scheduler уже занят stalled
  request;
- ready output/flush progression может продолжаться, даже если input/scheduler остаются заняты
  другим request path;
- pre-await work inside document-sync or barrier-related futures MAY существовать, но MUST NOT
  монополизировать тот же async task, что обслуживает transport reader или output writer;
- task-isolation MUST сохранять existing single-owner `poll_ready()/call()` semantics, а не
  заменять её конкурентными вызовами в несколько owners.

#### Scenario: Stalled scheduler branch не мешает позднему cancel быть классифицированным

- **GIVEN** scheduler уже держит stalled request branch до dispatch
- **AND** затем transport получает новый `$/cancelRequest`
- **WHEN** transport runtime продолжает работу
- **THEN** reader продолжает read/decode/classify нового control request
- **AND** cancel не застревает только потому, что stalled scheduler branch живёт на том же async
  task

#### Scenario: Ready response flush не стоит за unrelated scheduler stall

- **GIVEN** один request уже подготовил user-facing response и готов к output/flush progression
- **AND** другой request всё ещё держит scheduler path в stalled state
- **WHEN** transport runtime продолжает работу
- **THEN** ready response flush progresses independently
- **AND** output path не ждёт завершения unrelated scheduler stall только из-за same-task topology
