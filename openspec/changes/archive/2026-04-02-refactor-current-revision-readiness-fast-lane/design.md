## Контекст
Change `refactor-lsp-document-sync-slot-release` решает transport-level starvation от долгоживущих `didOpen/didChange` futures. Incident bundle `2026-03-23T08:03:23Z` показывает, что после этого bottleneck сместился глубже:
- медиана `service_future_to_first_poll_wait_ms` упала с `5857ms` в bundle `2026-03-22T16:19:59Z` до `80ms`, значит transport admission действительно улучшился;
- request `50` имеет `service_future_to_first_poll_wait_ms=1ms`, но всё равно умирает как `prepare_timeout@prepare_guard` после `3030ms` на `wait_for_file_version`;
- request `57` проходит `prepare`, уже видит `observed_file_version=9`, но завершает `exact_deadline` с `head_ready=false`, `exact_ready=false`;
- cumulative metrics показывают `intellisense_v2_runtime_wait_for_file_version_queue_wait_ms p99=9698ms` и `intellisense_v2_runtime_type_index_precompute_exec_ms p50=3485ms`, в то время как `intellisense_v2_runtime_queue_wait_interactive_ms p95=5ms`.

Следовательно, residual latency теперь определяется не admission в completion handler, а current-revision readiness path:
- `applied_version` продвигается слишком поздно после handoff;
- `CompletionHeadArtifact` для уже applied revision продолжает ждать slow enrich path;
- exact/type-index/diagnostics backlog всё ещё способен блокировать first current-revision response.

## Цели
- Зафиксировать отдельный fast-lane для same-file current-revision readiness: `applied_version` advance и `CompletionHeadArtifact` publish.
- Сохранить strict-latest, fail-closed и truthful observability semantics.
- Сделать residual regressions формально ловимыми через live LSP representative gate.

## Не-цели
- Не возвращать stale fallback вместо current-revision ответа.
- Не обещать ускорение full exact/type-index throughput само по себе.
- Не менять transport-slot contract из предыдущего change.
- Не добавлять новый observability payload, если existing bounded fields уже достаточны.

## Решение

### 1. Current-revision readiness выделяется в fast lane
После того как `didOpen/didChange` уже завершил свой transport future и зарегистрировал current-revision handoff, система должна считать interactive-critical минимумом для этого же файла:
- продвижение `applied_version` до latest requested revision;
- публикацию/queryability `CompletionHeadArtifact` той же revision.

Этот минимум MUST идти по readiness fast lane, который может вытеснять или supersede:
- same-file older-revision apply work;
- same-file `type_index_precompute`;
- same-file / older deferred diagnostics;
- other slow enrich stages, не являющиеся prerequisite для first current-revision response.

Идея не в том, чтобы сделать exact мгновенным, а в том, чтобы current revision как можно раньше стал:
- applied для bounded `wait_for_file_version`;
- head-ready для first non-empty completion response.

### 2. `applied_version` lag после handoff считается scheduler defect
Handoff по-прежнему не равен observable advance `applied_version`: transport future может вернуться раньше, чем runtime writer path завершит apply.

Но после handoff multi-second lag, из-за которого completion тратит почти весь путь в `wait_for_file_version`, больше не считается нормальным bounded outcome. Если latest apply остаётся в очереди позади background backlog, это defect readiness scheduler, а не "просто тяжёлый exact".

Практически это означает:
- latest same-file apply MUST иметь приоритет над background work, который не нужен для first current-revision response;
- superseded older-revision apply/head work MUST NOT блокировать newest revision;
- `prepare_timeout@wait_for_file_version` после handoff трактуется как regression, а не acceptable fail-closed path.

### 3. Head readiness отделяется от exact readiness
Если `min_file_version` уже наблюдается как `observed_file_version`, но `CompletionHeadArtifact` всё ещё не готов, completion не должен ждать exact/type-index так, будто head и exact являются одной стадией.

Новый контракт:
- `CompletionHeadArtifact` current revision MUST быть publishable/queryable до завершения slow exact/type-index/deferred diagnostics path;
- `ExactSemanticArtifact` MAY отставать и продолжать background upgrade;
- `exact_deadline` при `artifact_poll.observed_file_version == min_file_version` и `head_ready=false` считается regression readiness fast lane.

Иначе transport fix останется косметическим: запрос быстро попадает в handler, но дальше всё равно упирается в отсутствие first-response artifact для уже applied revision.

### 4. Gate должен мерить post-handoff readiness, а не только transport ingress
Representative real-module gate нужно расширить отдельным профилем `post-handoff readiness`:
- warmup phase не входит в measured set;
- measured set содержит не менее 10 completion samples;
- перед каждым measured sample gate делает changed-text `didChange` и затем member-access completion по тому же файлу через live LSP path;
- gate сохраняет и анализирует существующие authoritative поля:
  - `wait_for_file_version_runtime.queue_wait_ms`;
  - `prepare_details.min_file_version`;
  - `prepare_details.observed_file_version`;
  - `exact_wait.head_ready_before_wait`;
  - `exact_wait.artifact_poll`.

Для этого change gate MUST fail:
- если `p95(wait_for_file_version_runtime.queue_wait_ms) > 0.50 * interactive_wait_budget_ms`;
- если любой measured sample имеет `wait_for_file_version_runtime.queue_wait_ms > 4 * interactive_wait_budget_ms`;
- если любой measured sample завершился `prepare_timeout@wait_for_file_version` после same-file handoff;
- если любой measured sample завершился `exact_deadline` при `artifact_poll.observed_file_version == min_file_version` и `head_ready_before_wait=false`.

Такой gate различает четыре класса состояний:
- transport ingress regression;
- post-handoff apply backlog;
- post-apply head gap;
- slow exact upgrade после уже успешного first response.

## Рассмотренные альтернативы

### Увеличить interactive wait budget
Отклонено. Это лишь маскирует apply/head backlog и делает UX медленнее.

### Вернуть stale fallback
Отклонено. Это противоречит уже закреплённому strict current-revision / fail-closed contract и скрывает реальный defect readiness pipeline.

### Лечить проблему только ростом concurrency / permits
Отклонено. Это pressure relief, но не гарантирует приоритет newest same-file apply/head над low-value background work.

### Считать `CompletionHeadArtifact` производным exact readiness
Отклонено. Тогда любой exact backlog снова превращает first current-revision response в multi-second tail.

## Риски и trade-offs

### Риск: aggressive fast-lane замедлит diagnostics и exact upgrade
Смягчение:
- приоритет даётся только newest same-file readiness milestones;
- full exact/diagnostics остаются в фоне и продолжают исполняться после first response.

### Риск: усложнится lifecycle supersession/cancellation
Смягчение:
- change явно сохраняет latest-wins semantics и запрещает старым readiness tasks блокировать newest revision;
- gate и regression tests должны покрывать supersession.

### Риск: gate станет ловить client-side шум вместо server-side readiness
Смягчение:
- acceptance опирается только на authoritative server fields;
- client probes остаются вспомогательными и не используются как единственное доказательство.

## Acceptance-направление
- Completion после same-file `didChange` больше не проводит seconds-scale время в `wait_for_file_version` из-за post-handoff backlog.
- Current revision достигает `applied_version` и `CompletionHeadArtifact` fast path раньше slow exact/type-index/deferred diagnostics.
- `prepare_timeout@wait_for_file_version` и post-apply `head_ready=false` `exact_deadline` становятся gate-failing regressions.
- Strict current-revision / fail-closed semantics остаются без stale substitute.
