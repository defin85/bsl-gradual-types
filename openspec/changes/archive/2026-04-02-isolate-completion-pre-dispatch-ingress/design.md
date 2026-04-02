## Context

`stabilize-completion-front-edge` уже сделал completion traces request-bound и убрал ambiguity между client probes и server timeline. После этого свежий incident bundle `2026-03-27T18:17:21Z` показывает новый остаточный seam:

- completion traces `1-4` получают `client_before_transport_dominant`, хотя server-side `dispatch_to_request_context_wait_ms`, `transport_to_service_future_wait_ms`, `service_future_to_first_poll_wait_ms` и `turn_wait` там остаются около `0-1ms`;
- `transport_adapter.rs` ждёт `service.poll_ready()` до того, как request классифицируется как completion;
- `jsonrpc_dispatch_received_at_ms` фиксируется только в `DispatchContextService::call()`, то есть после readiness wait;
- extension считает `client_to_transport_wait_ms` как `transport_received_at_ms - lsp_request_started_at_ms`, поэтому этот wait сейчас может включать не только клиентский ingress, но и server backlog до dispatch.

Следовательно, next fix должен одновременно:

1. сделать observability truthful для окна `adapter read -> dispatch`;
2. убрать сам pre-dispatch head-of-line blocking для completion;
3. не переписывать заново существующий post-dispatch completion handoff и не размывать fail-closed/cancel semantics.

Для этого change architecture decision уже зафиксирован: целевой путь — `reader -> single-owner scheduler + strict priority lanes`. Варианты `instrumentation-only`, weighted/fair scheduler или client-side throttling НЕ рассматриваются как допустимый основной путь реализации.

## Versioning Note

- Текущий capability spec в `openspec/specs/bsl-intellisense-v2/spec.md` описывает shipped baseline, а не target-state этого change.
- Для данного change canonical target line остаётся непрерывной: public response version `18 -> 19`, versioned contract baseline `contracts/lsp-completion-timeline/v15 -> v16`.
- Следовательно, authoritative source of truth для target-state этого change — `proposal.md`, `design.md`, `tasks.md` и delta spec внутри `openspec/changes/isolate-completion-pre-dispatch-ingress/`.

## Goals / Non-Goals

### Goals

- Отделить настоящий client-side ingress от server pre-dispatch backlog в authoritative completion timeline.
- Изолировать completion admission от общего LSP backlog до `DispatchContextService::call()`.
- Сохранить existing completion handoff, current-revision guarantees, bounded fail-closed поведение и exactly-once terminal semantics.
- Добавить representative gates, которые ловят именно pre-dispatch starvation.

### Non-Goals

- Не делать общий rewrite всего LSP scheduling для всех методов.
- Не менять post-dispatch completion pipeline, exact-wait lifecycle и slot-release design.
- Не поднимать transport concurrency как substitute для архитектурного исправления.
- Не перекладывать проблему на клиентский throttling `documentSymbol` или другие heuristics.

## Decisions

### 1. Новый authoritative split вводится как отдельная ранняя adapter timestamp, а не как reinterpretation старых полей

Рекомендуемое решение: authoritative trace получает дополнительную раннюю server-side timestamp на transport adapter boundary сразу после decode/read request, плюс derived wait для окна `adapter read -> dispatch`.

Почему так:

- existing `transport_received_at_ms` уже используется как bounded server-edge marker и сейчас может совпадать с `jsonrpc_dispatch_received_at_ms`;
- изменение смысла существующего поля разрушило бы уже накопленные versioned contracts и extension projections;
- отдельный split позволяет доказать, где именно лежит wait: до dispatch, между dispatch и request context, или уже после создания service future.

Последствия:

- contract version поднимается `18 -> 19`;
- contiguous baseline поднимается `v15 -> v16`;
- derived consumers получают новый truthful server-side ingress slice без переписывания существующих post-dispatch metrics.

Семантика полей фиксируется явно:

- `adapter_read_at_ms` — earliest server-side ingress boundary на adapter seam, записанная сразу после успешного read/decode и до любого shared readiness blocking;
- `adapter_to_dispatch_wait_ms` — только server-side wait между `adapter_read_at_ms` и earliest dispatch boundary;
- existing `transport_received_at_ms` НЕ переосмысляется задним числом как ранняя adapter boundary и сохраняет legacy semantics для backward-compatible consumers.

### 2. Admission path ОБЯЗАТЕЛЬНО разбивается на `reader -> single-owner scheduler`

Выбранное решение: transport adapter больше не должен делать `poll_ready()` прямо в read loop до request classification. Вместо этого:

- `reader` только читает/декодирует JSON-RPC сообщения и сразу классифицирует method;
- `scheduler` остаётся единственным владельцем `poll_ready()/call()` для Tower service;
- completion и control requests попадают в собственные очереди до shared readiness wait;
- existing post-dispatch completion handoff остаётся downstream responsibility и не переписывается.

Граница ответственности фиксируется жёстко:

- `reader` делает только `read -> decode -> classify -> enqueue`;
- `reader` MUST NOT вызывать `poll_ready()`, `call()` или напрямую владеть readiness/backpressure state inner service;
- `scheduler` MUST единолично выбирать lane, ждать readiness, фиксировать dispatch boundary и вызывать `call()`;
- post-dispatch completion dispatcher/cancellation path остаётся отдельным downstream слоем и не дублируется новым ingress scheduler.

Почему так:

- Tower `Service` предполагает явную readiness/backpressure семантику; single-owner scheduler безопаснее, чем попытка распараллелить `poll_ready()` на несколько call sites;
- prior art с producer/scheduler split уже есть в `tower::buffer`, где producers не владеют service readiness напрямую;
- текущий bug живёт именно в окне до `call()`, поэтому лечить его только post-dispatch techniques недостаточно.

Этот выбор обязателен для данного change: `instrumentation-only` path может идти только как промежуточный подэтап внутри реализации, но не как завершённое состояние change.

### 3. Lane policy фиксируется как strict priority `control -> completion -> general`

Первый шаг намеренно остаётся минимальным и недвусмысленным:

- `$/cancelRequest` и shutdown/control traffic получают наивысший приоритет;
- `textDocument/completion` и completion-supporting document-sync notifications (`didOpen`/`didChange`/`didSave`/`didClose`) получают следующий приоритет, когда нужно сохранить current-revision handoff до последующего completion на том же transport path;
- весь остальной трафик идёт в `general` lane.

Почему так:

- свежая evidence показывает interactive pain именно на completion ingress;
- overly generic weighted scheduler сейчас только раздует scope change;
- strict priority достаточно, чтобы снять текущий bottleneck и сохранить change проверяемым.

Trade-off:

- general traffic может деградировать под completion storm сильнее, чем раньше;
- это осознанно принимается как follow-up risk, если новая evidence покажет starvation уже на general lane.

Следствие:

- bounded queue capacities и overload/backpressure policy должны быть зафиксированы сразу;
- fairness для `general` lane остаётся осознанно отложенным follow-up, а не скрытой частью этой реализации.

Queue/backpressure policy для этого change фиксируется так:

- `control` lane имеет выделенную bounded capacity и MUST NOT вытесняться lower-priority traffic, пока transport остаётся жив;
- `completion` lane bounded и может вытеснять только stale/superseded completion work по уже существующим rules, но MUST NOT деградировать в silent drop или перекидывание в `general`; reader использует bounded spillover того же класса, а после его насыщения обязан сохранять control reserved progress через fail-closed pre-dispatch `queue_rejected` для older queued completion вместо блокировки single reader; current-revision handoff для `didOpen`/`didChange`/`didSave`/`didClose`, уже прочитанных transport adapter'ом до completion на том же path, MUST оставаться в этом interactive admission path;
- `general` lane bounded и принимает на себя основной backpressure/блокировку admission для unrelated traffic, не забирая reserved progress у `control` и `completion`; unrelated id-less notifications MAY получать bounded traceable drop только после того, как completion-supporting handoff уже вынесен из этого lane;
- policy обязана быть явной и наблюдаемой: enqueue rejection/coalescing/supersession должны оставаться bounded, deterministic и traceable.

### 4. Queued cancellation должна оставаться first-class и exactly-once

Новый scheduler не должен превращать pre-dispatch queue в blind FIFO. Если completion request уже стоит в очереди, а control lane получает `$/cancelRequest`, система должна:

- найти соответствующий queued completion;
- пометить его cancelled до dispatch;
- не допустить позднего publish после того, как cancel уже признан terminal.

Почему так:

- LSP cancellation не требует обязательной остановки compute, но требование exactly-once terminal semantics остаётся;
- без queued cancellation новый scheduler может сам создать stale interactive tail и ухудшить overlap behaviour.

Terminal contract для pre-dispatch cancellation фиксируется явно:

- queued completion, отменённый до dispatch, MUST завершаться ровно одним terminal result на request/response path и MUST NOT оставаться hanging;
- для LSP/JSON-RPC terminal response MUST использовать cancellation semantics `RequestCancelled`;
- authoritative completion timeline для такого request MUST публиковать bounded terminal outcome `cancelled`;
- если dispatch не произошёл, payload MUST NOT выдумывать post-dispatch timestamps/derived waits.

### 5. Acceptance должен измерять pre-dispatch backlog отдельно от existing first-poll metrics

Существующие gates полезны, но они смотрят в основном на `service_future_to_first_poll_wait_ms`, `transport_to_handler_wait_ms` и handler phases. Для нового bottleneck этого недостаточно.

Новый representative gate должен:

- запускать same-file mixed load через real LSP transport;
- собирать `adapter read -> dispatch` evidence отдельно от existing post-dispatch waits;
- fail-ить, если `p95(adapter_to_dispatch_wait_ms)` превышает interactive budget или любой measured sample выходит за `4x` budget;
- fail-ить, если completion pre-dispatch wait снова становится dominant из-за concurrent general traffic;
- fail-ить, если queued cancel до dispatch нарушает exactly-once terminal response или публикует fabricated post-dispatch fields;
- не маскировать regression как purely client-side ingress, когда authoritative adapter split уже доказывает server backlog.
- так как shipped change-specific wrapper также запускает blocking representative-matrix perf gate, delivery обязан сохранять согласованность с текущей shared runtime latency policy для соседних user-facing semantic queries (`members`, `type_at_position`), а не оставлять drift между transport-side remediation и общим interactive runtime contract.

## Alternatives Considered

### A. Добавить только observability fields без scheduler changes

Отклонено как недопустимое конечное состояние change. Это улучшит truthfulness, но не уберёт user-visible latency на default path.

### B. Просто поднять `DEFAULT_LSP_TRANSPORT_CONCURRENCY_LEVEL`

Отклонено. Это tuning/backpressure relief, а не root-cause fix для shared pre-dispatch seam.

### C. Расширить existing post-dispatch completion handoff на другие методы

Отклонено. Текущий bottleneck появляется до `call()`, поэтому post-dispatch isolation не закрывает дефект.

### D. Исправлять только `documentSymbol`

Отклонено как слишком узко для текущей evidence. Bundle показывает `documentSymbol` среди contenders, но сам seam живёт на общем adapter/service readiness path и может затрагиваться и другим general traffic.

### E. Делать client-side throttling outline/auxiliary методов

Отклонено. Это ухудшает truthfulness источника проблемы и перекладывает server-side scheduling defect на extension heuristics.

## Risks / Trade-offs

- Новый scheduler легко сломать по части exactly-once/cancel semantics.
  - Mitigation: single-owner dispatch, focused queued-cancel tests, reuse existing post-dispatch handoff invariants.
- Strict priority `control -> completion -> general` может увеличить starvation risk для general lane.
  - Mitigation: change явно scoped на completion pain; bounded queue policy обязательна; если появится новая evidence, fairness станет отдельным follow-up, а не скрытой complexity в этом change.
- Новый observability split повышает contract churn.
  - Mitigation: contiguous version bump `19/v16`, explicit degradation rules и focused extension tests на old/new payload.
- Может оказаться, что после truthful split значимая часть остаточной задержки действительно вне backend.
  - Mitigation: even then этот change остаётся полезным, потому что либо снимает реальный server seam, либо окончательно доказывает, что он больше не доминирует.

## External References

- Tower `Service`: https://docs.rs/tower/latest/tower/trait.Service.html
- Tower `buffer`: https://docs.rs/tower/latest/tower/buffer/index.html
- LSP 3.17 completion: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
- LSP 3.17 cancellation support: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#cancellation-support
- VS Code `CompletionItemProvider`: https://code.visualstudio.com/api/references/vscode-api#CompletionItemProvider
