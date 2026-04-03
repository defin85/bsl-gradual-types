# Change: снизить latency exact completion IR через shared revision flight и cooperative cancellation

## Почему

Incident bundle `2026-04-03T13:48:00Z` показал, что после truthful `query_bundle` attribution bottleneck уже не прячется в ingress и не сидит в `turn_wait`:

- `completion-trace-2`: `total=3249ms`, из них `query_bundle_ir_query=3079ms`;
- `completion-trace-1`: `total=2605ms`, из них `query_bundle_ir_query=2599ms`, и stale request замечает cancel только в конце длинного IR tail;
- `completion-trace-3`: `head_hit`, `total=1ms`.

Это значит, что первый current-revision response уже здоров, а exact path по-прежнему тратит секунды внутри IR query.

Локальная архитектура уже имеет общие `derived_cache` и revision-bound `ir` singleflight, но current-revision prewarm и interactive request path входят в exact IR через разные orchestration точки. В результате один и тот же revision key может греться фоном и одновременно считаться в request path, а superseded compute может продолжать жечь CPU до конца большого blocking closure.

Нужен отдельный remediation change: не новый observability слой, а снижение реальной server latency exact path.

## Что меняется

- фиксируем в `bsl-intellisense-v2`, что exact IR work для одного revision key MUST шариться между request path и background prewarm, а не запускаться параллельно из-за разных entrypoint'ов;
- требуем latest-only поведение для same-file exact prewarm: более новая revision supersede-ит старую warm job без publish stale artifacts;
- требуем cooperative cancellation checkpoints внутри exact IR/program-facts build, а не только вокруг всего blocking closure;
- запрещаем публикацию partial/stale exact artifacts и partial IR в shared cache при cancelled/superseded unwind;
- добавляем focused acceptance/tests/perf evidence для shared-flight reuse, superseded abort и отсутствия duplicate exact compute на одном revision key.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `analysis-v2/src/lib/snapshots.rs`
  - `analysis-v2/src/lib/analysis_api.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - focused backend/runtime tests and representative perf/readiness gates

## Non-Goals

- не решать client-side transport/post-response gap;
- не добавлять stale/degraded semantic fallback ради latency;
- не считать "увеличить blocking pool" или "переписать executor" выполнением change без shared-flight reuse и bounded stale abort внутри exact path.
