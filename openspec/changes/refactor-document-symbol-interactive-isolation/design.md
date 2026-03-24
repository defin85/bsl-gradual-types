## Контекст
Предыдущие два change уже закрыли два реальных bottleneck-класса:
- `refactor-current-revision-readiness-fast-lane` устранил post-handoff backlog между `didChange` и current-revision readiness;
- `refactor-completion-prepare-lightweight-exact-split` убрал mandatory heavy prepare из completion first-response path.

Новый incident bundle `2026-03-24T10-10-36Z` показывает, что основной residual stop-the-world эффект теперь живёт рядом с completion, а не внутри него:
- completion request `40` имеет `service_future_to_first_poll_wait_ms=18009`, но `server_handler_exec_ms=156`;
- completion request `54` имеет `service_future_to_first_poll_wait_ms=1` и `prepare.route=head_hit`, что подтверждает работоспособность current canonical completion path;
- cumulative metrics того же процесса показывают `intellisense_v2_drilldown_stage_latency_ms_origin_lsp_operation_document_symbol_stage_runtime_wait_for_file_version p95=14951ms` при `count=9`.

Одновременно сервер по умолчанию объявляет `documentSymbol` capability, а handler идёт через strict stateful prepare:
- `document_symbol_provider: true` в `InitializeResult`;
- `lsp_document_symbol()` использует `prepare_lsp_stateful_operation_v2(...)`.

Следовательно, основной нерешённый root cause сейчас такой: companion IDE request для Outline/Breadcrumbs способен занимать LSP admission path и starving interactive completion до входа в handler.

## Цели
- Изолировать `documentSymbol` как auxiliary outline path, чтобы он не мог задерживать interactive first response.
- Сохранить usable Outline/Breadcrumbs без требования multi-second strict current-version wait.
- Добавить observability и regression gate, которые детерминированно ловят outline-induced starvation.

## Не-цели
- Не переделывать весь LSP scheduling для всех методов сразу.
- Не менять strict current-revision semantics interactive completion.
- Не закрывать здесь secondary `turn_wait`, если он останется после снятия ingress starvation.
- Не связывать change с detached immutable snapshot архитектурой.

## Решение

### 1. `documentSymbol` считается auxiliary companion request
`textDocument/documentSymbol` не является частью canonical interactive semantic first-response contract. Это companion surface для IDE navigation/outline.

Из этого следуют два правила:
- `documentSymbol` MUST NOT удерживать interactive completion/hover/signatureHelp/definition за счёт strict current-version wait;
- корректность `documentSymbol` определяется пригодностью для Outline/Breadcrumbs, а не правом блокировать user keystroke path.

Этот change сознательно scoped только на `documentSymbol`, потому что именно он подтверждён authoritative metrics как текущий starvation source. Возможное расширение на другие auxiliary методы остаётся отдельным follow-up после новой evidence.

### 2. `documentSymbol` получает bounded serving states
Для одного файла `documentSymbol` должен уметь завершаться одним из трёх bounded outcomes:
- `current_ready`: структура для requested revision готова и возвращается сразу;
- `latest_ready`: requested revision ещё не materialized для symbol tree, но есть наиболее свежая готовая структура того же файла;
- `unavailable`: ни current-ready, ни latest-ready структура недоступны в bounded auxiliary policy.

`latest_ready` допустим только как outline/navigation payload и MUST NOT:
- masquerade-ить interactive semantic truth;
- использоваться completion/hover/signatureHelp/definition как substitute;
- скрывать факт lag через отсутствие observability attribution.

Практический смысл: если requested revision symbol tree не готов, сервер должен быстро выбрать `latest_ready` или `unavailable`, а не держать transport/service future на multi-second `wait_for_file_version`.

### 3. Admission и execution нужно изолировать от interactive path
Одной только bounded serving policy недостаточно, если `documentSymbol` всё ещё занимает те же admission slots, что и completion.

Новый контракт:
- `documentSymbol` MUST обслуживаться через auxiliary request class;
- auxiliary class MUST NOT потреблять interactive reserve, когда есть interactive waiters;
- completion/hover/signatureHelp/definition MUST сохранять гарантированный first-poll progress даже при активном outline refresh;
- older same-file `documentSymbol` requests MAY supersede/cancel when a newer refresh arrives.

Implementation detail намеренно не фиксируется на уровне OpenSpec:
- это может быть отдельная transport/admission lane;
- ранний demux до heavy prepare;
- auxiliary semaphore/classification внутри существующего request pipeline;
- комбинация этих техник.

Но observable contract фиксируется жёстко: outstanding `documentSymbol` не должен больше превращаться в multi-second completion ingress stall.

### 4. Outline refresh должен быть latest-wins и coalesced
VS Code может дёргать outline companion path сериями вокруг `didChange`/`didSave`. Если сервер честно исполняет каждый такой refresh, он сам создаёт backlog низкой ценности.

Поэтому для `documentSymbol` нужен latest-wins contract:
- per-file newer refresh supersede-ит older refresh, если старый ещё не принёс user-visible value;
- save/edit churn coalesce-ится до newest meaningful outline refresh;
- cancellation/supersession фиксируются observability как bounded outcome, а не теряются бесследно.

Это снижает pressure ещё до тяжёлой работы и убирает ложную ценность из хвоста очереди.

### 5. Acceptance должен мерить mixed load, а не только completion in isolation
Текущие completion gates хороши для `didChange`/readiness/exact path, но не доказывают non-interference со стороны auxiliary outline traffic.

Нужен отдельный representative mixed-load gate:
- real-module profile;
- на каждой measured итерации выполняются `didChange`/`didSave`, затем outline refresh (`documentSymbol`), затем completion на том же файле;
- gate анализирует authoritative server-side поля:
  - completion `service_future_to_first_poll_wait_ms`;
  - completion `transport_to_handler_wait_ms`;
  - completion route/outcome;
  - `documentSymbol` outcome class (`current_ready` / `latest_ready` / `unavailable` / `superseded`);
- gate fail-ит, если auxiliary outline traffic снова делает interactive completion ingress-dominant.

## Рассмотренные альтернативы

### Просто ещё раз увеличить transport concurrency
Отклонено. Это pressure relief, но не меняет того факта, что low-value auxiliary requests делят admission path с interactive requests.

### Сохранить strict current-version wait для `documentSymbol`
Отклонено. Incident уже показывает, что такой подход превращает Outline companion request в user-visible completion blocker.

### Вернуть stale semantic fallback для completion
Отклонено. Это нарушает уже принятый strict current-revision contract и маскирует реальный starvation defect.

### Сразу обобщить решение на все auxiliary LSP methods
Отклонено как первый шаг. Авторитетная evidence сейчас указывает именно на `documentSymbol`; новый change должен остаться минимальным и проверяемым.

## Риски и trade-offs

### Риск: Outline временно будет отставать от latest requested text
Это допустимый trade-off, если lag bounded и явно относится только к auxiliary navigation surface. Interactive semantic truth от этого не деградирует.

### Риск: Admission isolation окажется слишком локальной и не снимет starvation полностью
Смягчение:
- representative gate должен смотреть именно `service_future_to_first_poll_wait_ms`, а не только completion outcome;
- если после снятия `documentSymbol` starvation останется secondary culprit, он будет виден отдельно.

### Риск: `latest_ready` превратится в скрытый stale substitute
Смягчение:
- change явно ограничивает `latest_ready` только outline/navigation surface;
- observability обязана различать `current_ready` и `latest_ready`.

## Relationship with соседними changes
- Это follow-up после `refactor-current-revision-readiness-fast-lane` и `refactor-completion-prepare-lightweight-exact-split`: они лечили completion path, а не companion request starvation рядом с ним.
- Это orthogonal к `refactor-current-revision-head-detached-snapshot`: detached read model не исправит admission starvation сам по себе, поэтому смешивать оба изменения в один change нельзя.
