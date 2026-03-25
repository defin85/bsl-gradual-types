## Контекст
`Completion Timeline v11` уже дал важный bounded cut для сегмента `service_future_created -> first_poll -> first_wake`. Этого хватило, чтобы в incident bundle от `2026-03-25T09:40:08Z` доказать сам симптом:
- future completion request создаётся быстро;
- затем остаётся неполленной секунды;
- после первого poll handler/runtime tail работает быстро.

Но текущий contract всё ещё не отвечает на следующий практический вопрос: какой класс server-side нагрузки был видим, пока request future не получала первый poll. На сегодня operator вынужден читать код `RequestContextService`, понимать `Server::serve(...).concurrency_level(...)` и сопоставлять это с incident bundle вручную.

## Goals / Non-Goals
- Goals:
  - Дать следующий bounded diagnostic cut для сегмента `service_future_created -> first_poll`.
  - Перенести этот cut через already-shipped completion surfaces без guessed blocker claims.
  - Сохранить low-cardinality / bounded payload и fail-closed semantics.
- Non-Goals:
  - Не чинить сам starvation/latency regression.
  - Не вводить executor-wide tracing или новый unbounded debug stream.
  - Не делать full rewrite observability/perf pipeline.
  - Не менять probe schema или correlation policy.

## Решения
### 1. Добавить `v12` contender attribution как bounded snapshot, а не как "точный виновник"
Новый contract не должен обещать больше, чем сервер действительно может доказать. Поэтому новый объект описывает не конкретный blocking request, а bounded contender class, наблюдаемый server-side в окне `service_future_created -> first_poll`.

Рабочее имя payload-поля: `first_poll_contention_attribution`.

Object остаётся bounded и несёт только low-cardinality facts:
- `contender_class`
- `uri_scope`
- `inflight_count`
- `oldest_inflight_age_ms`
- `concurrency_level`

Где:
- `contender_class` различает хотя бы `document_sync`, `completion`, `other_request`, `other_notification`, `mixed`, `none_visible`, `unavailable`;
- `uri_scope` различает хотя бы `same_uri`, `other_uri`, `mixed`, `unavailable`.

Это позволяет incident handoff сказать "сервер видел document-sync contention на том же URI" или "конкуренты вообще не были видимы", но не подменять это claim'ом про точный root cause без доказательства.

### 2. Источник truth: bounded in-flight snapshot в server request context
Минимальный полезный путь лежит не через новый API, а через расширение уже существующего request-context instrumentation вокруг LSP service futures.

Нужен bounded registry/snapshot для in-flight LSP requests/notifications, из которого completion trace сможет:
- взять contender class;
- оценить `uri_scope` относительно своего `uri`;
- перенести aggregate facts (`count`, `oldest age`, `concurrency level`) в момент наблюдаемого first poll gap.

Важно: change не требует полного scheduler trace и не требует точного mapping "какой future блокировала именно эту". Достаточно truthful server-visible contender snapshot.

### 3. Fail-closed semantics важнее "полезной" догадки
Если snapshot unavailable или противоречив, payload не должен invent data. Вместо guessed blocker нужно использовать bounded `unavailable` или `mixed` semantics.

Это особенно важно для случаев, где:
- tracker видит несколько классов конкурентов;
- tracker не может честно определить `uri_scope`;
- request future long-wait видна, но конкуренты уже исчезли к моменту snapshot.

### 4. Existing completion surfaces показывают новый cut, но не переоценивают его
Completion Timeline panel, clipboard export и request-centric incident bundle summary должны:
- показывать новый `v12` attribution рядом с existing `v11` first-poll / first-wake split;
- явно деградировать на `v11`;
- формулировать derived handoff только как server-visible contender fact;
- не превращать `document_sync` / `completion` class в claim про точный blocking request, request id или URI.

## Альтернативы
- Полный executor-wide tracing.
  - Отклонено: слишком большой scope и operational cost для текущего incident-driven change.
- Новый custom request поверх текущего timeline.
  - Отклонено: problem already belongs to authoritative completion trace; новый surface только усложнит transport и деградацию.
- Использовать только client probes / correlation.
  - Отклонено: bundle уже показал `ambiguous` и `timestamp_mismatch`; client-side data не может быть единственным source of truth для server-side pre-poll contention.

## Риски / компромиссы
- Риск: registry начнёт тащить high-cardinality method/uri data в payload.
  - Mitigation: spec ограничивает payload bounded vocab и aggregate numbers.
- Риск: operator воспримет contender class как "доказанного виновника".
  - Mitigation: user-facing wording фиксируем как "видимый contender class", а не exact blocker.
- Риск: instrumentation окажется полезной только для части traces.
  - Mitigation: explicit `none_visible|unavailable` semantics лучше, чем silent omission или guessed root cause.

## План миграции
1. Поднять spec delta до `v12`.
2. Добавить bounded server-side contender snapshot в request-context instrumentation.
3. Протянуть новый object в completion timeline payload и versioned contract baseline.
4. Довести `v12` facts до panel / clipboard / incident bundle.
5. Зафиксировать деградацию на `v11` тестами и manual evidence.
