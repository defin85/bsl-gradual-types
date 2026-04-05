## Контекст
После `refactor-completion-current-revision-readiness-fast-lane` transport и handoff seams перестали объяснять completion latency, а interleaved interactive apply backlog был снят. Но свежий incident bundle `2026-04-05T15:40:13Z` показал другой остаточный gap.

В immediate same-file post-edit/save окне:
- completion traces `1..4` завершаются `prepare_timeout@wait_for_file_version`;
- traces `1` и `5` доходят до `snapshot_with_deps` только на границе budget и завершаются `prepare_timeout@snapshot_with_deps`;
- при этом truthful ingress/egress остаются почти нулевыми.

Позже тот же файл уже ведёт себя иначе:
- один sample проходит readiness и упирается в `query_bundle_pool_wait`;
- hot trace полностью bounded.

Значит текущая проблема больше не в transport starvation и не в общем save-triggered backlog, а в front-edge окне между registered same-file handoff и первой полностью prepared current-revision path.

## Цели
- Зафиксировать front-edge current-revision readiness window как отдельный regression surface.
- Требовать, чтобы первые completion сразу после `didChange/didSave` не умирали в `prepare_timeout` только из-за front-edge publication/readiness lag.
- Развести front-edge `prepare_timeout` и cold `query_bundle_pool_wait`.
- Сохранить strict current-revision truth и fail-closed semantics.

## Не-цели
- Не менять transport ingress/output handoff path.
- Не оптимизировать `query_bundle_pool_wait` и другой cold query-body path.
- Не вводить stale или detached substitute для current revision.
- Не открывать новый extension/UI scope без свежего прямого контрдоказательства.

## Решения

### 1. Front-edge readiness становится отдельной boundary
После same-file `didChange/didSave` handoff completion может войти в prepare слишком рано относительно fully prepared current-revision path. Это окно нужно считать отдельным remediation target.

Если completion в этом окне завершает `prepare_timeout`, outcome считается regression front-edge readiness window, а не допустимым bounded fail-closed результатом.

### 2. `wait_for_file_version` и `snapshot_with_deps` входят в один front-edge surface
Новый bundle показывает две формы одного и того же symptom:
- timeout прямо на `wait_for_file_version`;
- timeout на `snapshot_with_deps`, когда wait уже съел почти весь budget и real runtime queue wait не объясняет проблему.

Поэтому remediation scope не должен ограничиваться только `wait_for_file_version`. Для operator-facing acceptance оба исхода относятся к одному front-edge gap между handoff и первой fully prepared current-revision path.

### 3. Gate должен бить именно по immediate post-edit window
Representative profile должен моделировать не “поздний warm completion после churn”, а первые completion samples почти сразу после same-file `didChange/didSave`.

Gate обязан:
- fail-ить на любом `prepare_timeout` в этом front-edge окне при healthy truthful transport seams;
- отдельно report-ить successful-but-cold samples, где readiness уже прошёл, а latency ушла в `query_bundle_pool_wait`.

### 4. Cold `query_bundle_pool_wait` остаётся отдельным follow-up
Trace `6` показывает, что после успешного readiness может оставаться другой bottleneck. Этот change не должен размывать scope и превращаться в оптимизацию downstream pool/query path.

## Alternatives Considered

### Считать `prepare_timeout@snapshot_with_deps` отдельной runtime saturation проблемой
Отклонено. В observed bundle такие traces происходят в том же immediate post-edit window, что и `prepare_timeout@wait_for_file_version`, при почти нулевых truthful seams. Operationally это один front-edge gap.

### Сразу переходить к оптимизации `query_bundle_pool_wait`
Отклонено. Bundle показывает, что pool wait появляется уже после успешного readiness и не объясняет fail-closed traces.

### Переоткрыть transport/UI investigation
Отклонено. New evidence снова подтверждает, что truthful transport seams healthy.

## Риски и trade-offs

### Риск: remediation размоет strict current-revision semantics
Смягчение:
- stale substitute запрещён;
- fail-closed policy сохраняется;
- success критерий завязан на truthful current-revision path, а не на подмену ответа.

### Риск: gate снова будет conflating readiness и downstream cold cost
Смягчение:
- front-edge profile валится только на `prepare_timeout`;
- `query_bundle_pool_wait` сохраняется как отдельный diagnostic bucket.

### Риск: immediate front-edge profile окажется слишком чувствительным
Смягчение:
- acceptance привязывается к healthy truthful seams и same-file handoff;
- если front-edge timeout повторяется в таком профиле, это остаётся unacceptable regardless of later warm recovery.

## Migration / Rollout
1. Уточнить spec и acceptance для front-edge readiness window.
2. Реализовать remediation на backend prepare/publication boundary.
3. Переснять front-edge live gate и incident bundle.
4. Если front-edge timeout исчезает, а `query_bundle_pool_wait` остаётся, открывать отдельный change уже под downstream pool/query cost.

## Open Questions
- Достаточно ли ускорить observable front-edge publication внутри текущего pipeline, или нужен отдельный bounded fast path между same-file handoff и `snapshot_with_deps` availability?
