## Контекст

`refactor-completion-query-bundle-root-cause-attribution` уже сделал root cause truthful. Bundle `2026-04-03T13:48:00Z` показывает:

- ingress healthy (`adapter_to_dispatch_wait_ms=0..2`, `turn_wait=0`);
- `head_hit` path быстрый (`1ms`);
- exact invoked path доминирует внутри `query_bundle_ir_query` (`~2.6-3.1s`);
- cancelled stale request наблюдает cancel только в конце длинного IR tail.

Это уже не observability-проблема. Нужен remediation на exact IR path.

При этом база для дешёвого выигрыша уже существует:

- host snapshots делят общий `db` и `derived_cache`;
- `analysis.ir(...)` уже имеет revision-bound cache/singleflight contract;
- current-revision prewarm уже умеет заранее трогать `analysis.ir(file_id)`.

Но сегодня prewarm и request path входят в IR разными путями. Из-за этого дедупликация между ними не гарантирована, а coarse cancellation вокруг большого blocking closure не помогает быстро остановить устаревший IR build.

## Versioning Note

Этот change не требует нового публичного completion timeline contract сам по себе. Цель здесь не новый observability payload, а реальное снижение latency exact path и elimination duplicate compute.

## Goals

- Убрать duplicate same-revision exact IR compute между prewarm и request path.
- Снизить CPU tail stale exact work после supersession/cancel.
- Сохранить exact correctness: без stale publish, без partial cache, без degraded substitute.

## Non-Goals

- Не чинить client-side latency gap.
- Не подменять remediation новым coarse fallback path.
- Не считать отдельный executor rewrite обязательной частью change.

## Решения

### 1. Request path и prewarm используют один revision-bound exact IR flight

Для одного `(file_id, file_version, deps_id, settings_id)` exact IR work должен иметь один canonical entrypoint.

Это означает:

- background prewarm больше не делает direct bypass мимо request-path singleflight;
- если prewarm уже стал leader для exact IR той же revision, interactive request attach-ится follower'ом;
- если interactive request уже запустил exact IR flight, prewarm не стартует duplicate compute, а reuse-ит этот flight.

Цель не в том, чтобы любой compute шарился глобально, а в том, чтобы один и тот же exact IR одного revision key не выполнялся дважды только из-за разных orchestration path.

### 2. Same-file prewarm остаётся latest-only

Prewarm для файла должен подчиняться тому же latest-wins contract, что и interactive serving:

- более новая revision supersede-ит warm job для старой revision;
- superseded warm job не публикует stale exact artifact как latest;
- newer revision может запускать свой flight, не дожидаясь, пока старая warm job доест весь stale tail.

Это не обещает hard preemption в каждой точке IR builder, но обещает, что obsolete prewarm перестаёт считаться полезной работой и boundedly сворачивается на внутренних checkpoints.

### 3. Cooperative cancellation протягивается внутрь IR/program-facts build

Текущие coarse checkpoints вокруг целого blocking closure недостаточны: long IR build может крутиться секунды после того, как request уже superseded.

Новый contract требует checkpoint-ов внутри:

- AST -> IR build;
- крупных exact facts / type-inference passes;
- body-level или statement-batch boundaries, где можно дешево проверить cancel state.

Гранулярность не должна опускаться до "проверять каждый AST node". Нужны крупные, но достаточно частые checkpoints, чтобы stale exact tail перестал быть seconds-scale blind compute.

### 4. Partial exact artifacts никогда не записываются как successful cache entry

Cancellation внутри build допустима только если:

- partial IR не попадает в shared `derived_cache` как valid artifact;
- incomplete exact facts не публикуются follower'ам как success;
- stale revision не становится latest source of truth после unwind.

Иначе ускорение будет куплено ценой corruption cache contract, что недопустимо.

### 5. Acceptance должен доказывать reuse и bounded stale abort

Просто "latency стала лучше на глаз" недостаточно. Change должен оставить проверяемые доказательства:

- same-revision prewarm/request reuse;
- отсутствие duplicate exact IR build для одного revision key;
- superseded exact build не публикует stale/partial artifact;
- representative churn path больше не держит seconds-scale stale exact compute без внутренних checkpoints.

## Alternatives Considered

### Просто увеличить blocking pool / permits

Отклонено. В bundle queue wait почти нулевой. Проблема не в admission saturation, а в duplicate compute и coarse cancellation внутри exact build.

### Оставить cancellation только вокруг blocking closure

Отклонено. Именно это и даёт multi-second stale tail внутри `query_bundle_ir_query`.

### Переписать exact compute на отдельный executor сразу

Отклонено как слишком широкий первый шаг. Сначала нужен дешёвый выигрыш: shared revision flight, latest-only prewarm и cooperative checkpoints внутри текущей архитектуры.

## Риски и Trade-offs

### Риск: checkpoint-ы ухудшат happy-path throughput

Смягчение:

- ставить их по крупным batches, а не по каждому node;
- проверять на representative large-module traces, что overhead меньше выигрыша от stale abort.

### Риск: unified flight усложнит lifecycle prewarm/request

Смягчение:

- оставить один canonical revision key и один leader/follower lifecycle;
- покрыть attach/detach cleanup focused tests.

### Риск: partial artifact утечёт в cache при cancelled unwind

Смягчение:

- записывать exact IR/facts в cache только после terminal successful build;
- делать cancel/fail path отдельным terminal outcome, а не "почти success".
