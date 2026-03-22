## Контекст
Incident bundle `2026-03-22T16:19:59Z` показывает два разных класса проблем:
- long `service_future_created -> first poll` waits у completion requests (`5857ms`, `14754ms`, `6040ms`);
- отдельный artifact-readiness gap, где current revision eventually становится `head_hit`, но слишком поздно для bounded wait.

Этот change сознательно фокусируется на первом классе: transport-level starvation до первого `poll()`.

Текущее объяснение опирается на код и telemetry:
- `backend/src/bin/lsp_server/main.rs` запускает `Server::new(...).serve(service)` без явного повышения `concurrency_level`;
- `tower-lsp` transport выполняет request futures через ограниченный `buffer_unordered(self.max_concurrency)`, так что futures могут быть созданы сразу, но ждать первый `poll()`, пока не освободится slot;
- `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` рано публикует current revision, но затем inline ждёт slow `parse snapshot build` и только потом завершает `didOpen/didChange` service future.

По bundle это выглядит консистентно:
- `service_future_to_first_poll_wait_ms` для completion близок к `parse_snapshot_build_incremental p95`;
- completion dispatcher внутри handler часто уже `ready`, значит хвост сидит раньше completion handler;
- obsolete/cancelled completion requests после предыдущих фиксов уже перестали тратить handler time зря, но pre-poll backlog остаётся.

## Цели
- Зафиксировать архитектурный контракт: `didOpen/didChange` не держат LSP transport slot, пока идут slow background стадии.
- Сохранить truthful current-revision semantics: new revision становится видимой до completion, но slow parse/head/exact work продолжается отдельно и не маскируется fallback-ом.
- Добавить acceptance gate, который ловит regressions именно на реальном transport path.

## Не-цели
- Не перепроектировать current-revision `CompletionHeadArtifact`, который сейчас публикуется как побочный продукт full IR.
- Не менять completion result contract и не добавлять stale substitute.
- Не лечить проблему простым увеличением `tower-lsp` concurrency.

## Решение

### 1. Short-lived document-sync service future
`didOpen/didChange` service future должна делать только transport-critical работу:
- принять и валидировать входной payload;
- обновить `latest_received` / shadow state;
- поставить current-revision `SetFile` в analysis runtime writer path для той же `file_version`;
- зарегистрировать или перевыставить background work для parse/head/exact/diagnostics;
- завершиться, освободив transport slot.

Slow stages после handoff MUST выполняться вне document-sync transport future. Иначе они продолжают конкурировать с interactive completion не по CPU budget, а уже на более ранней transport boundary.

`applied_version` в рамках этого change сохраняет базовую семантику: это revision, уже применённая в analysis runtime через current-revision `SetFile` / `SetFileWithSnapshot`. Change не переопределяет `applied_version` как readiness `CompletionHeadArtifact`, `ExactSemanticArtifact` или diagnostics publish.

Current-revision handoff здесь сознательно означает enqueue/register writer work, а не обязательное наблюдаемое продвижение `applied_version` к моменту возврата `didOpen/didChange`. После возврата document-sync future `received_version` и shadow state уже отражают новую requested revision, но interactive path по-прежнему должен дождаться `applied_version` через bounded `wait_for_file_version`.

#### Stage boundary
Inline до возврата `didOpen/didChange`:
- payload validation и построение updated text / parser edits;
- обновление shadow state и `latest_received_file_versions_v2`;
- current-revision `SetFile` enqueue/register в runtime writer path;
- enqueue / supersede background parse-head-exact-diagnostics work.

Background после возврата `didOpen/didChange`:
- `parse snapshot build`;
- `SetFileWithSnapshot`;
- completion head publish/reuse from parse snapshot;
- exact/type-index precompute;
- deferred diagnostics.

### 2. Slow work переносится за transport boundary, но не за correctness boundary
После handoff background tasks всё ещё обязаны:
- не публиковать результат для stale `(file_version, deps_id, settings_id)`;
- уважать supersession/cancellation;
- не выдавать semantic truth другой revision под видом current revision;
- сохранять observability пригодной для расследования.

Иными словами, change меняет lifecycle, но не ослабляет strict-latest и fail-closed semantics.

### 3. Representative gate должен идти через реальный transport
Текущие direct service harness tests полезны, но не ловят starvation между `service_future_created` и первым `poll()`.

Нужен gate, который:
- поднимает live LSP server path;
- генерирует `didChange-burst` на representative large module;
- затем измеряет member-access completion;
- отдельно оценивает `service_future_to_first_poll_wait_ms`, first-response availability и exact-upgrade latency.

Gate обязан падать, если completion остаётся функционально успешным только после seconds-scale pre-poll starvation, потому что это именно transport regression, а не acceptable slow exact upgrade.

Для этого change `didChange-burst` gate должен быть operationalized, а не qualitative only:
- warmup phase не входит в measured set;
- measured set содержит не менее 10 completion samples;
- gate fail-ит при `p95(service_future_to_first_poll_wait_ms) > 250ms`;
- gate fail-ит при любом measured sample с `service_future_to_first_poll_wait_ms > 1000ms`, если overshoot атрибутирован pending document-sync futures, а не client-side ingress.

## Рассмотренные альтернативы

### Увеличить `tower-lsp` concurrency
Это pressure relief, а не root-cause fix. Long-lived document-sync futures останутся long-lived и будут продолжать отнимать transport slots, только при более высоком лимите.

### Вставить `yield` внутри текущего `didChange`
`yield` не меняет fact, что тот же service future остаётся живым и продолжает занимать transport slot до терминального завершения. Проблема lifecycle остаётся.

### Компенсировать transport хвост completion-side fallback-ом
Это ломает already-shipped fail-closed/current-revision contract и маскирует architectural defect пользовательским degraded result.

## Риски и trade-offs

### Риск: background handoff усложнит version/cancellation lifecycle
Смягчение:
- change прямо требует сохранить strict token validation и supersession semantics;
- acceptance должен покрывать и correctness, и latency.

### Риск: transport fix сам по себе не снимет все `exact_deadline`
Смягчение:
- это известный separate issue и он остаётся вне scope;
- gate всё равно продолжает проверять first-response availability на representative module.

### Риск: change переобещает ускорение type resolution
Смягчение:
- contract change ограничен pre-poll transport backlog;
- handler-internal `IR` / `type resolution` latency остаётся отдельной осью производительности и не считается дефектом этого change по умолчанию.

### Риск: регрессия спрячется в synthetic tests
Смягчение:
- change требует отдельный real-transport gate, а не только прямой вызов service layer.

## Acceptance-направление
- После burst `didChange` completion больше не проводит seconds-scale время между `service_future_created` и первым `poll()` только из-за pending document-sync service futures.
- `didOpen/didChange` возвращают transport control до завершения slow parse/head/exact stages.
- current-revision correctness и fail-closed semantics не ослабляются.
- Representative real-module gate различает pre-poll transport backlog и exact-upgrade latency и валит change при возврате slot-retention regression.
