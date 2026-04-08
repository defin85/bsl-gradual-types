## Контекст
После `refactor-completion-front-edge-readiness-window` transport seams и `prepare_timeout` перестали быть доминирующим symptom. Но acceptance-review checked-in report показал другой residual failure mode:

- front-edge traces идут через `shadow_current_revision_fast_path`;
- дальше request spends весь bounded budget в `wait_exact_type_index`;
- trace заканчивается fail-closed с `type_index_wait_outcome=deadline`;
- public reason уходит в generic `missing_semantic_index`;
- representative gate при этом остаётся зелёным, даже если `measured_successful_traces=0`.

Иными словами, regression surface не исчезла, а сменила имя и стала менее truthful для operator-facing evidence.

## Цели
- Устранить front-edge same-file `exact_deadline` как допустимый steady-state outcome при healthy truthful transport seams.
- Не позволять masking: front-edge exact wait exhaustion не должна схлопываться в generic `missing_semantic_index` без explicit attribution.
- Сделать representative gate строгим: он обязан видеть хотя бы один successful current-revision sample в том же immediate post-edit/save profile.
- Сохранить strict current-revision truth и fail-closed semantics.

## Не-цели
- Не оптимизировать весь downstream `query_bundle_pool_wait` path в рамках этого change.
- Не ослаблять requirement до "любой bounded fail-closed acceptable".
- Не подменять current revision stale snapshot-ом или detached substitute path.

## Решения

### 1. `exact_deadline` в front-edge окне считается той же regression surface, а не acceptable substitute
Если same-file completion после зарегистрированного handoff входит в front-edge fast path, а затем исчерпывает bounded `wait_exact_type_index`, это остаётся readiness regression.

Change не должен просто переименовывать такой outcome в `wait_not_ready`/`missing_semantic_index`. Нужно либо реально довести request до successful current-revision sample, либо сохранить explicit regression attribution, пригодную для gate и observability.

### 2. Gate обязан видеть успех, а не только отсутствие старого symptom name
Representative acceptance больше не может считаться выполненной, если:
- `prepare_timeout=0`, но
- все measured traces fail-closed, и
- ни один measured sample не проходит readiness до query path.

Operator-facing contract для этого change: immediate front-edge profile должен породить хотя бы один successful current-revision sample. Только после этого downstream `query_bundle_pool_wait` может считаться отдельным bucket, а не дырой в readiness remediation.

### 3. Front-edge evidence должно быть durable и truthful
Проверка не должна опираться только на устные выводы по внешнему incident bundle. В репозитории должен оставаться checked-in artifact или проверяемый report summary, по которому видно:
- zero front-edge `prepare_timeout`;
- zero front-edge `exact_deadline`;
- non-zero successful traces;
- отдельный status по cold `query_bundle_pool_wait`.

## Alternatives Considered

### Разрешить all-fail-closed front-edge profile как acceptable bounded behavior
Отклонено. Это прямо противоречит intent предыдущего change и превращает remediation в relabeling regression.

### Оставить `exact_deadline` как допустимую public reason
Отклонено. Для immediate same-file handoff window это тот же unresolved readiness gap, а не нормальная деградация.

### Сразу перенести scope на downstream `query_bundle_pool_wait`
Отклонено. Пока front-edge profile не даёт ни одного successful sample, downstream pool wait нельзя считать следующим bottleneck по данному сценарию.

## Риски и trade-offs

### Риск: stricter gate станет флейковым
Смягчение:
- success requirement должен проверяться на representative profile с уже доказанно healthy transport seams;
- budget/criteria нужно завязать на truthful current-revision sample, а не на arbitrary total latency without attribution.

### Риск: explicit `exact_deadline` attribution ухудшит зелёность gate до фактического remediation
Смягчение:
- это ожидаемо и полезно: gate должен fail-ить, пока real root cause не закрыт.

### Риск: remediation попытается скрыть проблему агрессивным background prewarm
Смягчение:
- stale substitute запрещён;
- success должен оставаться truthful current-revision outcome;
- observability обязана сохранять distinction между front-edge readiness и downstream cold query cost.

## Migration / Rollout
1. Уточнить spec так, чтобы hidden `exact_deadline` больше не считался acceptable bounded outcome.
2. Исправить request/runtime path так, чтобы immediate post-edit/save profile давал хотя бы один successful current-revision sample.
3. Ужесточить representative gate и report.
4. Переснять durable evidence и только после этого считать front-edge remediation завершённой.
