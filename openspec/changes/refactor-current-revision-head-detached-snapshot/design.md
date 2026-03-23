## Контекст
Текущая архитектура уже имеет сильные стороны:
- один source of truth за writer thread;
- отдельный current-revision `CompletionHeadArtifact`;
- atomic `deps + index` bundle;
- operation-aware scheduling.

Но даже после split-prepare completion first-response path остается завязан на live runtime boundary. Это приемлемо как ближайший remediation step, но не является лучшей долгосрочной boundary shape.

Если системе понадобится более слабая связность между producer и consumer, то нужен не shared `AnalysisV2`, а отдельный detached immutable read model.

## Цели
- Опубликовать current-revision completion head как detached immutable canonical read model.
- Снизить coupling между writer-owned runtime state и first-response completion consumers.
- Сохранить exact semantic truth в отдельном exact path.
- Сделать publication/read lifecycle проверяемым через acceptance и observability.

## Не-цели
- Не превращать весь runtime в full immutable solution graph.
- Не убирать writer как source of truth.
- Не заменять exact path detached snapshot’ом.
- Не использовать detached snapshot как excuse для stale semantics.

## Решение

### 1. Публикуется отдельный detached immutable head snapshot
После того как current-revision head truth построен из canonical IR, система публикует отдельный immutable read model, пригодный для first completion response.

Этот snapshot:
- не является `AnalysisV2`;
- не удерживает writer-owned mutable host state;
- может безопасно читаться несколькими readers;
- keyed по `(file_id, file_version, deps_id, settings_id)`.

### 2. Snapshot содержит только bounded first-response payload
Detached snapshot должен нести только то, что нужно для current-revision first response:
- bounded owner/type hints;
- candidate skeletons или другой минимальный completion payload;
- consistency ids и revision identity.

Он не должен превращаться в скрытый second exact artifact или в serialized whole semantic world.

### 3. Producer и consumer разделяются явным контрактом
Producer responsibility:
- построить canonical current-revision head truth;
- опубликовать detached snapshot;
- invalidation/supersession по latest-wins semantics.

Consumer responsibility:
- читать detached snapshot для first response;
- использовать exact path только там, где нужен full semantic contract.

### 4. Publication lifecycle должен быть явным
Detached snapshot MUST:
- invalidated при смене `file_version`;
- invalidated при смене `deps_id` или `settings_id`;
- superseded более новой revision того же файла;
- не использовать stale detached payload другой revision как substitute.

### 5. Change зависит от появления нормального completion boundary
Detached snapshot не должен внедряться прямо в текущий жирный generic prepare. Практически он предполагает, что split-prepare или эквивалентная feature-specific boundary уже существует.

Иначе система получит новый артефакт, но оставит старую inappropriate request boundary.

## Рассмотренные альтернативы

### Оставить lightweight path на live runtime reads навсегда
Отклонено как долгосрочный ceiling. Это сохраняет лишнюю связанность consumer path с runtime ownership-моделью.

### Использовать shared `AnalysisV2` как published snapshot
Отклонено. Текущий `AnalysisV2` не является detached immutable read model и не должен masquerade-ить под него.

### Делать full immutable solution/runtime graph
Отклонено как слишком широкий и дорогой scope относительно цели completion first-response path.

## Риски и trade-offs

### Риск: snapshot schema разрастется до скрытого second semantic world
Смягчение:
- bounded contents;
- строгий scope только для first completion response;
- exact truth остается отдельной canonical boundary.

### Риск: publication/invalidation lifecycle станет хрупким
Смягчение:
- key по `(file_id, file_version, deps_id, settings_id)`;
- явный latest-wins supersession contract;
- representative gate на real-module churn path.

### Риск: detached snapshot введут слишком рано, до split-prepare
Смягчение:
- change явно рассматривается как отдельный следующий шаг после нормализации completion boundary.

## Acceptance-направление
- Current-revision head snapshot публикуется как detached immutable read model.
- Completion first-response path читает его без зависимости от shared runtime snapshot abstraction.
- Exact semantic path остается отдельным и truthful.
- Representative gate различает publication/read regressions detached snapshot и slow exact upgrade.
