## Контекст
Incident bundle `2026-03-26T19:02:29Z` показывает новый остаточный failure mode после `refactor-completion-turn-wait-lifecycle`.

Наблюдаемое поведение:
- один trace (`request=32`) копит `16378ms` до first poll и видит same-file contender `completion[phase=turn_wait]` того же возраста;
- другой trace (`request=44`) получает первый `poll()` почти сразу, но затем проводит `3457ms` внутри handler с dominant `turn_wait`;
- `turn_wait` сейчас ожидается inline внутри completion handler, а LSP сервер по-прежнему ограничен `DEFAULT_LSP_TRANSPORT_CONCURRENCY_LEVEL=16`.

Это означает, что проблема уже не сводится к orphaned pre-active waiter или ошибке telemetry cleanup. Даже когда current request admitted честно и telemetry правдива, passive `turn_wait` внутри service future продолжает занимать transport slot. В burst/overlap сценариях это превращает completion lifecycle в ingress bottleneck.

Архитектурная проверка текущего runtime показывает дополнительное ограничение:
- `tower-lsp` transport ограничивает ingress через concurrency slots, которые живут до завершения `Service<Request>::Future`;
- обычный LSP response для входящего request сейчас привязан к результату этого future;
- `Client`/loopback path предназначен для server-to-client messages и не даёт готового “late response sink” для завершения уже принятого inbound request.

Следовательно, чистый refactor только `impl_completion.rs`/dispatcher не гарантирует release transport slot. Для выполнения change нужен project-owned handoff на transport/service boundary.

Concrete seam для этого change:
- `backend/src/bin/lsp_server/main.rs` больше не должен напрямую вызывать `tower_lsp::Server::serve(service)` для default completion path;
- новый модуль `backend/src/bin/lsp_server/server/transport_adapter.rs` должен владеть project-local scheduling adaptation;
- `backend/src/bin/lsp_server/server/mod.rs` должен экспортировать этот transport adapter как единственный transport entry point для binary через `server::serve_with_completion_handoff(...)`.

## Цели
- Перестать удерживать LSP transport admission slot на default event-driven completion path только ради passive `turn_wait`.
- Сохранить existing completion semantics: same-file latest-wins, explicit cancel, bounded terminal outcomes, no late publish.
- Сохранить truthful observability: operator должен отличать ingress backlog от off-transport completion wait.
- Поймать этот failure mode отдельным deterministic regression и representative real-module gate.

## Не-цели
- Не менять fairness/policy для всех request classes.
- Не лечить проблему приоритизацией, bump concurrency или дополнительным fallback.
- Не подменять root-cause fix размытым “background everything” без сохранения normal LSP response semantics.
- Не переписывать весь LSP runtime, если scoped adaptation default event-driven completion path поверх текущего `tower-lsp` достаточно.

## Решения

### 1. Passive `turn_wait` должен происходить после completion-specific handoff на transport/service boundary
Текущий blind spot не в том, что dispatcher выдаёт неверный turn outcome, а в том, что request уже admitted в `tower-lsp` service future и затем пассивно ждёт чужой turn inline.

Новый change фиксирует другой boundary:
- request-context, correlation и cancellation hook захватываются на service/admission path как и раньше;
- default event-driven completion request MUST передаваться в отдельный completion-owned handoff до начала длительного `turn_wait`;
- handoff MUST происходить в `server::serve_with_completion_handoff(...)` / `server::transport_adapter`, потому что перенос `await` только внутри existing completion handler не освобождает transport slot;
- passive wait за dispatcher turn или older same-file owner MUST происходить вне transport slot retention.

Это completion-scoped fix. Он не требует глобально менять обработку всех LSP methods, но требует локально владеть transport boundary для default completion path.

`main.rs` остаётся composition root и должен только собирать `service + socket + adapter config`, а не содержать completion-specific scheduling logic.

### 2. После handoff должен существовать ровно один lifecycle owner для response/cancel/cleanup
Post-handoff path нельзя строить как “просто ещё один background task” без ownership contract.

Нужен единый completion-owned lifecycle owner, который:
- владеет `request_id`, terminal outcome и правом отправить ровно один terminal response;
- держит cancellation token и shutdown cleanup;
- остаётся подчинённым тому же dispatcher/epoch authority, что и текущий inline path;
- не допускает split-brain между pre-active cleanup, active cleanup и late publish checks.

Это основной mitigation против race windows `handoff -> cancel`, `handoff -> supersede`, `turn_wait -> shutdown`.

### 3. Same-file lifecycle contract не должен деградировать из-за handoff
Перенос passive wait за transport boundary не должен ломать уже поставленные guarantees из `refactor-completion-turn-wait-lifecycle`.

После handoff must remain true:
- newer same-file request всё ещё supersedes older request boundedly;
- explicit `$/cancelRequest` всё ещё доходит до request, который ещё не получил dispatcher turn;
- stale request всё ещё не публикует поздний completion response;
- request id и terminal outcome остаются сопоставимыми между timeline, client probes и LSP response path;
- request получает не более одного terminal response даже при гонке между cancel/supersede/shutdown.

Dispatcher остаётся единственным authority для ordering/latest-wins. Handoff не даёт deferred task права самостоятельно решать, что completion может стать active или publishable.

### 4. Observability должна отделять ingress backlog от off-transport completion wait
После handoff оператору уже нельзя объяснять весь latency profile только через `service_future_created -> first_poll`.

Новый contract должен позволять увидеть:
- bounded ingress до handoff/admission;
- момент handoff/release transport slot;
- отдельный completion-owned wait после handoff;
- stale contenders в `phase=turn_wait`, если они ещё существуют;
- отсутствие invented timestamps и совместимость со старыми payload через явную graceful degradation.

### 5. Acceptance должен fail-ить и на handler-resident passive wait, и на race windows после handoff
Предыдущий gate ловил stranded pre-active contenders и seconds-scale pre-first-poll backlog. Новый incident показывает ещё один bad state: current request first-poll bounded, но затем сидит seconds-scale в handler на passive `turn_wait`.

Следовательно, новый acceptance layer должен fail-ить, если:
- current request удерживает transport/handler path на multi-second passive `turn_wait`;
- same-file overlap снова превращает completion lifecycle в ingress bottleneck;
- deferred completion теряет ownership и допускает double-response/double-cleanup в гонке cancel/supersede/shutdown;
- representative evidence больше не позволяет отличить off-transport wait от transport-slot retention.

Acceptance boundary для race windows считается закрытой только если tests доказывают:
- `cancel` и `supersede` не приводят к двойному terminal response для одного `request_id`;
- shutdown не допускает поздний publish после bounded cleanup handoff owner;
- completion owner терминируется ровно один раз даже при одновременном `cancel + supersede` сигнале.

## Рассмотренные альтернативы

### Локальный refactor только completion handler / dispatcher
Отклонено. В текущем `tower-lsp` transport ingress slot удерживается до завершения `Service<Request>::Future`, поэтому простой перенос `turn_wait` в helper future внутри existing handler не даёт release transport slot.

### Поднять `concurrency_level`
Отклонено. Это ослабляет симптом, но не меняет fact, что passive completion wait занимает transport slot.

### Повысить priority completion поверх других request classes
Отклонено. В текущем incident bottleneck уже создаёт сам completion `turn_wait`, а не только соседний traffic.

### Оставить handler inline и лечить только telemetry/gates
Отклонено. Последний bundle уже показывает реальный `server_handler_exec_ms=3457` с dominant `turn_wait`; это не просто observability artifact.

### Полностью заменить/fork-нуть LSP runtime для всех методов
Отклонено как baseline option. Это допустимый fallback только если scoped adaptation default event-driven completion path поверх текущего `tower-lsp` не позволяет сохранить normal response semantics.

## Риски и trade-offs

### Риск: completion handoff усложнит response path и ownership terminal response
Смягчение:
- держать boundary локальным для default event-driven completion;
- ввести один lifecycle owner для post-handoff request;
- отдельными tests/gates проверять exactly-once terminal response и no-late-publish.

### Риск: timeline contract станет более сложным
Смягчение:
- добавлять только bounded fields, которые действительно нужны для отделения ingress от off-transport wait;
- сохранить explicit degradation для старых payload.

### Риск: project-local transport adaptation создаст upgrade drift относительно upstream `tower-lsp`
Смягчение:
- минимизировать diff от текущего transport/service scheduling;
- ограничить adaptation только той веткой, которая нужна default completion path;
- зафиксировать concrete seam (`main.rs` -> `server::serve_with_completion_handoff(...)` -> `server::transport_adapter`) и не размазывать transport logic по `impl_completion.rs` и `core.rs`;
- явно держать ownership change в runbook/agent docs и representative evidence.

### Риск: gate станет flaky из-за реального overlap timing
Смягчение:
- удерживать deterministic harness для unit/live regression;
- на representative gate опираться на server-side stage breakdown и bounded outcome, а не только на client wall-clock.

### Риск: background/deferred completion task переживёт shutdown или потеряет cleanup
Смягчение:
- использовать explicit task tracking и cancellation propagation;
- требовать bounded terminal cleanup на shutdown/cancel path;
- fail-closed, если handoff owner не может безопасно завершить response path.
