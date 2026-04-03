## Контекст

После truthful `query_bundle` attribution стало видно, что серверный hot path и perceived UX tail больше не совпадают один в один.

Bundle `2026-04-03T13:48:00Z` показывает:

- `request=41`: `server=3249ms`, `client=9918ms`;
- `request=36`: `server=2605ms`, `client=10759ms`;
- derived summary уже выделяет `client_to_transport_wait_ms` и большой post-response хвост, но не может уверенно сказать, чья это часть.

Корень blind spot:

- `response_sent_at_ms` ставится в handler как response-ready boundary;
- фактический stdio write/flush завершает response позже в transport adapter;
- client probe сегодня знает dispatch и terminal resolution, но не raw transport receive.

Итог: post-response gap смешивает как минимум server egress, transport-after-flush и client-after-receive delay.

## Versioning Note

Текущий target-state после `refactor-completion-query-bundle-root-cause-attribution` — contract `v20` и contiguous baseline `v17`.

Этот change строится поверх него и целится в:

- `response.version = 21`;
- `contracts/lsp-completion-timeline/v18`.

`v21` сохраняет `v20` query-body taxonomy и existing additive ingress splits. Новый change добавляет только post-handler egress / receive split.

## Goals

- Разложить opaque post-response tail на bounded evidence buckets.
- Сохранить additive/backward-compatible semantics для существующих полей.
- Сделать incident bundle пригодным для разбора без guessed "server виноват" выводов.

## Non-Goals

- Не лечить сам server IR bottleneck.
- Не доказывать заранее конкретного виновника до появления новых bounded clocks.
- Не вводить новый telemetry backend или unbounded logging.

## Решения

### 1. `response_sent_at_ms` не переосмысляется; server egress split добавляется отдельно

Существующий `response_sent_at_ms` уже shipped и означает handler-local response-ready boundary. Этот change не должен задним числом превращать его во "flush completed".

Поэтому `v21` вводит отдельные additive поля:

- `response_flush_completed_at_ms`;
- `response_ready_to_flush_wait_ms`.

Если flush boundary недоступна на конкретном runtime path, сервер omits оба поля и не выдумывает approximate timestamp.

### 2. Client probe обязан отличать raw receive от promise resolution

Сегодня probe знает, когда request стартовал и когда promise resolved, но этого недостаточно для gap attribution.

Нужны отдельные bounded milestones:

- client enter;
- LSP dispatch;
- raw transport response receive;
- LSP promise resolve;
- client terminal.

Если raw receive boundary технически недоступна на части runtime paths, probe обязан фиксировать это как explicit bounded unavailable marker, а не silently подменять receive временем promise resolution.

### 3. Human-readable surfaces публикуют split buckets, а не один opaque tail

Когда `v21` payload и correlated probe дают полный split, derived surfaces должны переносить:

- `client_to_transport_wait_ms`;
- `response_ready_to_flush_wait_ms`;
- `transport_to_client_receive_wait_ms`;
- `client_receive_to_resolve_wait_ms`;
- `client_post_response_ms`.

Existing opaque umbrella вроде `server_to_client_post_response_ms` может оставаться compatibility summary, но MUST NOT быть единственным evidence bucket, если новый split доступен.

### 4. Incident summary остаётся fail-closed

Если server flush boundary или client raw receive boundary недоступны, summary обязан:

- явно отметить, какой именно split unavailable by design;
- не агрегировать missing часть как точный server-side или client-side виновник;
- оставить request summary валидным и bounded.

### 5. `v20` degradation остаётся явной

Для `v20` и старых probe paths surfaces:

- не выдумывают `response_flush_completed_at_ms`;
- не выдумывают raw client receive;
- прямо говорят, что post-response gap unresolved by design для этой версии evidence.

## Alternatives Considered

### Считать `response_sent_at_ms` фактическим flush completion

Отклонено. Это сломает смысл уже shipped поля и подменит blind spot ложной точностью.

### Ограничиться только extension-host profiler без новых bounded clocks

Отклонено. Профиль полезен как follow-up, но без transport/flush split он не даёт request-local authoritative evidence.

### Добавить только server flush split без client receive split

Отклонено. Тогда хвост всё равно останется смешанным между transport-after-flush и extension-host lifecycle.

## Риски и Trade-offs

### Риск: raw receive boundary трудно зацепить в `vscode-languageclient`

Смягчение:

- держать новый split bounded и debug-oriented;
- допускать explicit unavailable marker там, где runtime seam недоступен.

### Риск: surfaces станут сложнее для чтения

Смягчение:

- публиковать новый split только когда есть evidence;
- сохранять compatibility umbrella как вторичную сводку, а не как единственный verdict.

### Риск: additive `v21` поля придётся поддерживать долго

Смягчение:

- не переиспользовать их для unrelated telemetry;
- ограничить change только completion timeline / incident bundle contracts.
