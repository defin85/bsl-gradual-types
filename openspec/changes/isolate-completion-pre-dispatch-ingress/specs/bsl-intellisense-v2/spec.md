## MODIFIED Requirements

### Requirement: LSP предоставляет versioned per-request completion timeline контракт (MUST)
LSP MUST предоставлять server-driven custom request `bsl.getCompletionTimeline` с contract version `19`.

Для VS Code extension в текущей архитектуре этот контракт MUST быть доступен через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
Per-request timeline payload MUST формироваться на стороне LSP и MUST NOT требовать клиентской реконструкции из логов, incident summary или агрегированных observability-метрик.

Репозиторий MUST поддерживать versioned contract baseline `contracts/lsp-completion-timeline/v16`, синхронизированный с текущим authoritative payload и его bounded field-set.

В этом change `transport_received_at_ms` сохраняет существующую legacy semantics и MUST NOT ретроактивно переосмысляться как ранняя adapter boundary.
Новый earliest server-side ingress split MUST публиковаться только через additive поля `adapter_read_at_ms` и `adapter_to_dispatch_wait_ms`.

VS Code extension MAY отображать отдельно captured local client-side completion probes рядом с server trace, и такой local-only debug stream MAY включать bounded cancellation hints, transport-phase diagnostics, result-shape diagnostics и overlap/drift diagnostics, но такой stream:
- MUST NOT менять contract version или shape server-generated payload;
- MUST NOT подменять server-generated stages, routes, causes, waiter states или outcomes;
- MUST оставаться отдельным UI-level stream, а не частью LSP timeline contract.

Контракт `v19` MUST включать:
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

Если `turn_attribution` присутствует, объект MUST оставаться bounded и MAY включать `dispatcher_resolution_latency_ms`, достаточный для отделения dispatcher-ready latency от остального ingress wait.

Если `prepare_details` присутствует, объект MUST оставаться bounded и MUST NOT вводить high-cardinality labels.
Для completion bottleneck drilldown этот объект MUST включать:
- `route` со значениями только из bounded vocabulary `head_hit|exact_hit` либо `null`;
- `fail_closed_cause` со значениями только из bounded vocabulary `prepare_timeout|exact_deadline` либо `null`;
- `progress` с coarse prepare phase и bounded offsets;
- optional bounded runtime drilldown для `wait_for_file_version`;
- optional bounded runtime drilldown для `snapshot_with_deps`;
- optional bounded `timeout_attribution`;
- optional `exact_wait`.

Если `server_edge_details` присутствует, объект MUST оставаться bounded и MUST включать:
- `transport_received_at_ms`;
- `transport_received_at_ms_provenance`;
- `pre_method_attribution_provenance`;
- `handler_entered_at_ms`;
- `response_sent_at_ms`;
- optional `cancel_observed_at_ms`;
- `transport_to_handler_wait_ms`;
- `server_handler_exec_ms`;
- optional `cancel_observed_after_handler_enter_ms`;
- optional `adapter_read_at_ms`;
- optional `adapter_to_dispatch_wait_ms`;
- optional `jsonrpc_dispatch_received_at_ms`;
- optional `dispatch_to_request_context_wait_ms`;
- optional `method_entered_at_ms`;
- optional `transport_to_method_wait_ms`;
- optional `method_prelude_exec_ms`;
- optional `service_scope_entered_at_ms`;
- optional `transport_to_service_scope_wait_ms`;
- optional `service_scope_to_method_wait_ms`;
- optional `service_future_created_at_ms`;
- optional `transport_to_service_future_wait_ms`;
- optional `service_future_to_scope_wait_ms`;
- optional `service_future_first_poll_entered_at_ms`;
- optional `service_future_to_first_poll_wait_ms`;
- optional `service_future_first_poll_outcome`;
- optional `service_future_first_wake_scheduled_at_ms`;
- optional `first_poll_to_first_wake_wait_ms`.

`transport_received_at_ms_provenance` MUST использовать только bounded vocabulary:
- `request_context_call_entry`;
- `jsonrpc_dispatch_received`.

`service_future_first_poll_outcome` MUST использовать только bounded vocabulary:
- `ready`;
- `pending`.

Если `adapter_read_at_ms` присутствует, это поле MUST обозначать earliest server-side adapter ingress boundary, записанную сразу после успешного read/decode transport message и до shared readiness/admission blocking.

Если `adapter_read_at_ms` присутствует, payload MUST включать и `adapter_to_dispatch_wait_ms`, чтобы pre-dispatch split не требовал ручного вычитания timestamp'ов.

Если `adapter_to_dispatch_wait_ms` присутствует, это поле MUST описывать только server-side wait между `adapter_read_at_ms` и earliest dispatch boundary и MUST NOT включать client-side ingress или post-dispatch wait.

Если `method_entered_at_ms` присутствует, payload MUST включать и `transport_to_method_wait_ms`, и `method_prelude_exec_ms`, чтобы ingress attribution можно было прочитать без ручного вычитания timestamp'ов.

Если `service_scope_entered_at_ms` присутствует, payload MUST включать и `transport_to_service_scope_wait_ms`, и `service_scope_to_method_wait_ms`, чтобы pre-method split оставался self-contained.

Если `service_future_created_at_ms` присутствует, payload MUST включать и `transport_to_service_future_wait_ms`, и `service_future_to_scope_wait_ms`, чтобы pre-service-scope split не требовал ручного вычитания timestamp'ов.

Если `jsonrpc_dispatch_received_at_ms` присутствует, payload MUST включать и `dispatch_to_request_context_wait_ms`, чтобы pre-request-context split не требовал ручного вычитания timestamp'ов.

Если `service_future_first_poll_entered_at_ms` присутствует, payload MUST включать и `service_future_to_first_poll_wait_ms`, чтобы first-poll split не требовал ручного вычитания timestamp'ов.

Если `service_future_first_wake_scheduled_at_ms` присутствует, payload MUST включать и `first_poll_to_first_wake_wait_ms`, чтобы first-wake split не требовал ручного вычитания timestamp'ов.

Если `transport_received_at_ms_provenance=jsonrpc_dispatch_received`, payload MUST включать `jsonrpc_dispatch_received_at_ms`, а `transport_received_at_ms` MUST совпадать с ним.

Если `transport_received_at_ms_provenance=request_context_call_entry`, payload MUST NOT выдумывать `jsonrpc_dispatch_received_at_ms` и `dispatch_to_request_context_wait_ms`.

Если request завершился до dispatch, payload MUST NOT выдумывать `jsonrpc_dispatch_received_at_ms`, `dispatch_to_request_context_wait_ms`, `transport_to_method_wait_ms` или `method_prelude_exec_ms`.

Если `service_future_first_poll_outcome=ready`, payload MUST NOT выдумывать `service_future_first_wake_scheduled_at_ms` и `first_poll_to_first_wake_wait_ms`.

Каждый stage entry MUST включать:
- `name`;
- `status` (`completed|cancelled|failed|skipped`);
- `started_offset_ms`;
- `duration_ms`.

#### Scenario: VS Code клиент получает server-generated payload без reconstruction
- **GIVEN** VS Code extension запрашивает completion timeline
- **WHEN** клиент вызывает `workspace/executeCommand` с `command: bsl.getCompletionTimeline`
- **THEN** LSP возвращает response контракта `v19` с server-generated traces
- **AND** клиент не строит authoritative server trace из raw logs, incident summary или p95/p99 агрегатов

#### Scenario: Pre-dispatch backlog виден отдельно от dispatch/request-context split
- **GIVEN** completion request был прочитан transport adapter, но dispatch в service задержался
- **WHEN** клиент читает `server_edge_details`
- **THEN** payload содержит `adapter_read_at_ms` и `adapter_to_dispatch_wait_ms`
- **AND** post-dispatch поля (`jsonrpc_dispatch_received_at_ms`, `dispatch_to_request_context_wait_ms`, `transport_to_method_wait_ms`) остаются отдельными bounded срезами

#### Scenario: Versioned contract baseline синхронизирован с shipped payload
- **GIVEN** authoritative completion timeline уже публикует contract `v19`
- **WHEN** репозиторий фиксирует versioned contract baseline для этой поверхности
- **THEN** `contracts/lsp-completion-timeline/v16` совпадает по bounded field-set с runtime payload
- **AND** policy/verification scripts валидируют именно `v19/v16`, а не более старую версию

#### Scenario: Legacy transport ingress field не переосмысляется задним числом
- **GIVEN** authoritative payload содержит новый adapter boundary split
- **WHEN** downstream consumer читает `server_edge_details`
- **THEN** `transport_received_at_ms` сохраняет legacy semantics
- **AND** earliest adapter ingress публикуется отдельно через `adapter_read_at_ms`

### Requirement: Human-readable completion ingress verdicts остаются truthful и positive-only (MUST)
Derived verdicts для `Completion Timeline` panel, clipboard и связанных extension projections MUST строиться только из уже имеющихся bounded latency fields и MUST NOT маркировать trace как ingress-bottleneck, если соответствующая ingress задержка отсутствует.

Derived verdict layer MUST:
- использовать только существующие bounded waits (`adapter_to_dispatch_wait_ms`, `transport_to_method_wait_ms`, `method_prelude_exec_ms` и, при наличии deterministic correlation в downstream consumer, `client_to_transport_wait_ms`);
- строить ingress verdict только при положительной доминирующей задержке;
- различать как минимум `adapter_before_dispatch_dominant`, `server_before_method_entry_dominant` и `handler_prelude_dominant`;
- MAY различать `client_before_transport_dominant`, если downstream projection уже имеет deterministic probe correlation и authoritative earliest server ingress boundary;
- не выводить generic ingress verdict только потому, что `0 >= 0` или потому что одна из задержек отсутствует.

#### Scenario: Adapter wait доминирует над dispatch-to-method и handler prelude
- **GIVEN** completion trace имеет положительный `adapter_to_dispatch_wait_ms`, который доминирует над `transport_to_method_wait_ms` и `method_prelude_exec_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `adapter_before_dispatch_dominant`
- **AND** trace не получает `client_before_transport_dominant` только из-за позднего dispatch timestamp

#### Scenario: Hot trace без положительного ingress wait не получает ingress verdict
- **GIVEN** completion trace имеет `adapter_to_dispatch_wait_ms=0`, `transport_to_method_wait_ms=0` и `method_prelude_exec_ms=0`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает ingress verdict
- **AND** trace не маркируется как `handler_prelude_dominant`

#### Scenario: Handler prelude доминирует над server-side waits
- **GIVEN** completion trace имеет положительный `method_prelude_exec_ms`, который доминирует над `adapter_to_dispatch_wait_ms` и `transport_to_method_wait_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** trace получает verdict `handler_prelude_dominant`
- **AND** trace не получает `adapter_before_dispatch_dominant`

### Requirement: Client-side ingress supplement остаётся fail-closed и deterministic (MUST)
Если extension-projection добавляет human-readable client-side ingress verdict поверх authoritative completion trace, такой verdict MUST появляться только при deterministic probe correlation и положительном доминирующем client-side wait до самой ранней authoritative server ingress boundary.

Проекция MUST:
- не создавать client-side ingress verdict для uncorrelated или ambiguous requests;
- использовать `adapter_read_at_ms` как server ingress boundary, если payload её содержит;
- использовать более поздний `transport_received_at_ms` только как backward-compatible fallback для старых payload'ов, где ранняя adapter boundary отсутствует;
- не использовать probe-only эвристики как substitute для authoritative server verdicts;
- сохранять trace валидным и server-centric, если client correlation недоступна.

#### Scenario: Pre-dispatch server backlog не публикуется как client-side ingress
- **GIVEN** request summary имеет deterministic correlation
- **AND** authoritative payload содержит положительный `adapter_to_dispatch_wait_ms`
- **AND** положительный wait до ранней adapter boundary не доказан
- **WHEN** extension строит human-readable verdicts
- **THEN** trace не получает verdict `client_before_transport_dominant`
- **AND** projection остаётся fail-closed по client-side supplement

#### Scenario: Legacy payload без adapter boundary сохраняет bounded fallback
- **GIVEN** request summary имеет deterministic correlation, но connected server возвращает более старый payload без `adapter_read_at_ms`
- **WHEN** extension строит human-readable verdicts
- **THEN** projection MAY использовать bounded legacy fallback на `transport_received_at_ms`
- **AND** verdict не публикуется, если deterministic client-side delay всё равно не доказан

### Requirement: Representative real-module gate проверяет current-revision first-response availability для completion (MUST)
Acceptance для архитектурных изменений completion MUST включать representative gate на реальном workspace module, а не только synthetic URI harness.

Этот gate MUST:
- открывать реальный модуль из representative large configuration;
- проверять отдельно `same-revision warm` member-access completion и `revision-churn` completion после нового `didChange` перед каждым measured sample;
- включать `didChange-burst` профиль через реальный LSP transport path, а не только прямой вызов service layer;
- отдельно учитывать `adapter_to_dispatch_wait_ms`, `service_future_to_first_poll_wait_ms`, first-response availability и exact upgrade latency;
- использовать warmup phase, которая не входит в measured set;
- собирать не менее 10 measured completion samples в `didChange-burst` профиле;
- fail-ить, если `p95(adapter_to_dispatch_wait_ms)` у measured completion samples выше `intellisense_v2_interactive_wait_budget_ms`;
- fail-ить, если любой measured sample имеет `adapter_to_dispatch_wait_ms > 4 * intellisense_v2_interactive_wait_budget_ms`;
- fail-ить, если completion после новой revision снова деградирует в `fail_closed`, несмотря на наличие current-revision canonical fast path;
- fail-ить, если успешный first response достигается только после seconds-scale pre-dispatch backlog, вызванного concurrent general LSP traffic.

#### Scenario: Real-module gate ловит возврат pre-dispatch completion starvation
- **GIVEN** gate отправляет `didChange` churn и concurrent general LSP traffic через live transport path
- **WHEN** measured completion samples снова получают seconds-scale wait до dispatch
- **THEN** gate завершается ошибкой, даже если completion позже становится `ok_non_empty`
- **AND** отчёт выделяет pre-dispatch backlog отдельно от post-dispatch first-poll и handler latency

## ADDED Requirements

### Requirement: Interactive completion admission изолирован от general LSP backlog до dispatch (MUST)
Система MUST изолировать `textDocument/completion` от unrelated general LSP traffic в окне между чтением request transport adapter'ом и dispatch в service pipeline.

Изоляция MUST обеспечивать:
- shared readiness/admission state MUST принадлежать одному scheduler owner; reader/producers MUST NOT вызывать `poll_ready()/call()` напрямую;
- completion request классифицируется и попадает в interactive admission queue до shared `poll_ready()` blocking для general traffic;
- general requests MUST NOT удерживать freshly-read completion request вне interactive admission queue только из-за общего readiness wait;
- control traffic (`$/cancelRequest`, shutdown-related flow) MAY preempt queued completion admission;
- queued completion cancellation MUST сохранять existing exactly-once terminal semantics, MUST возвращать ровно один terminal response и MUST NOT допускать late publish после признанного cancel.

#### Scenario: General request burst не блокирует completion до dispatch
- **GIVEN** transport adapter уже читает burst general requests, включая `textDocument/documentSymbol`
- **AND** на том же transport path приходит новый completion request
- **WHEN** сервер выбирает, что dispatch-ить дальше
- **THEN** completion попадает в interactive admission queue без ожидания завершения general readiness path
- **AND** authoritative trace не показывает seconds-scale `adapter_to_dispatch_wait_ms` только из-за concurrent general backlog

#### Scenario: Queued completion отменяется до dispatch без late publish
- **GIVEN** completion request уже стоит в pre-dispatch queue
- **AND** до его dispatch приходит matching `$/cancelRequest`
- **WHEN** scheduler обрабатывает control lane
- **THEN** queued completion помечается cancelled до dispatch
- **AND** сервер возвращает ровно один terminal response с cancellation semantics `RequestCancelled`
- **AND** authoritative trace публикует outcome `cancelled` без выдуманных post-dispatch timestamps
- **AND** система сохраняет exactly-once terminal semantics без поздней публикации completion result
