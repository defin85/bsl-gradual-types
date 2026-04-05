## Контекст
Последний representative incident bundle показал смену dominant root cause.

После `refactor-lsp-auxiliary-runtime-isolation`:
- `client_to_transport_wait_ms` больше не seconds-scale;
- `service_future_to_first_poll_wait_ms` остается около нуля;
- `response_output_handoff_send_wait_ms` больше не показывает backlog.

Но interactive completion после same-file edit/save churn все еще может завершаться `prepare_timeout@wait_for_file_version` ровно на bounded prepare budget. При этом в том же bundle видно отдельный cold-path sample, где readiness уже не проблема, а latency уходит в `query_bundle_ir_query` и `collect`.

Это означает, что transport starvation и current-revision readiness starvation больше не одно и то же. Следующий remediation step должен лечить readiness fast lane, не смешивая ее ни с UI/transport path, ни с long-term detached snapshot architecture, ни с cold semantic query optimization.

## Цели
- Зафиксировать post-edit/save churn как отдельный current-revision readiness regression surface.
- Требовать, чтобы current-revision handoff и observable readiness оставались bounded даже при same-file auxiliary churn.
- Развести acceptance между readiness regressions и cold `query_bundle_ir_query` latency.
- Сохранить strict current-revision truth: никакого stale substitute и никакой подмены `applied_version`.

## Не-цели
- Не менять truthful transport/handoff seams, если они уже остаются в бюджете.
- Не оптимизировать cold semantic/query-body path.
- Не использовать detached immutable snapshot как обязательный prerequisite для этого remediation.
- Не менять extension-side `bsl.getCurrentContext` contract или UI behavior.

## Решения

### 1. Post-edit/save churn становится отдельной readiness boundary
После current-revision handoff interactive completion должен различать две независимые фазы:
- readiness: `applied_version` и `CompletionHeadArtifact` для requested revision становятся наблюдаемыми в bounded budget;
- semantic body: после успешного readiness completion может пойти в `head_hit`, `exact_hit` или отдельный cold query-body path.

`prepare_timeout@wait_for_file_version` в representative same-file edit/save profile считается failure readiness fast lane, даже если в том же процессе существуют другие samples с дорогим `query_bundle_ir_query`.

### 2. Same-file auxiliary churn не должен вытеснять newest readiness
`didSave`, same-file outline/context refresh и прочий auxiliary churn могут продолжать существовать как representative load, но они не должны:
- откладывать observable advance `applied_version` newest requested revision;
- удерживать completion в `wait_for_file_version` до исчерпания bounded prepare budget;
- оправдывать fail-closed исход там, где truthful transport seams уже healthy.

Fix scope остается на backend readiness scheduling/publication boundary. Representative auxiliary load нужен как guard, а не как новое направление расследования.

### 3. Acceptance обязан разводить readiness timeout и cold query-body
Representative gate должен fail-ить на readiness regression отдельно от cold semantic cost:
- если sample умирает в `prepare_timeout@wait_for_file_version`, это readiness failure;
- если sample успешно проходит readiness и затем дорогой `query_bundle_ir_query`, это отдельный latency bucket и отдельная follow-up проблема.

Gate не должен позволять cold query-body объяснять readiness timeout и наоборот.

### 4. Long-term detached snapshot остается отдельным эволюционным треком
Active change про detached immutable current-revision head snapshot не отменяется, но и не блокирует этот remediation. Здесь нужна более узкая гарантия: существующий current-revision contract должен стабильно выдерживать representative edit/save churn уже сейчас.

## Alternatives Considered

### Сразу переходить к detached immutable snapshot
Отклонено как слишком широкий scope для текущего regression. Bundle показывает immediate gap в существующем readiness path, который нужно закрыть до следующей архитектурной эволюции.

### Сначала оптимизировать `query_bundle_ir_query`
Отклонено. Bundle показывает, что readiness timeout случается раньше cold semantic phase и должен рассматриваться отдельно.

### Вернуться к transport/UI investigation
Отклонено. Truthful ingress/egress seams уже остаются около нуля и не объясняют observed `prepare_timeout`.

## Риски и trade-offs

### Риск: попытка ускорить readiness сломает truthful current-revision semantics
Смягчение:
- `applied_version` не переопределяется как artifact readiness;
- stale semantic substitute запрещен;
- acceptance остается привязанной к current-revision truth.

### Риск: representative gate будет conflating readiness и cold body cost
Смягчение:
- отдельные pass/fail критерии для `prepare_timeout@wait_for_file_version`;
- отдельная отчетность для `query_bundle_ir_query`/`collect`.

### Риск: same-file auxiliary churn окажется не единственным offender
Смягчение:
- change фиксирует symptom surface, а не один конкретный implementation detail;
- rollback criterion простой: repeated `prepare_timeout@wait_for_file_version` при healthy truthful transport seams остается unacceptable.

## Migration / Rollout
1. Уточнить spec и representative acceptance для same-file edit/save churn.
2. Реализовать remediation на current-revision readiness boundary.
3. Переснять representative live gate и incident bundle.
4. Если readiness timeout исчезает, а cold `query_bundle_ir_query` остается, открывать следующий отдельный change уже под cold semantic latency.

## Open Questions
- Нужен ли отдельный readiness promotion path для save-triggered same-file refresh, или достаточно усилить существующую latest-wins publication discipline вокруг current-revision handoff?
