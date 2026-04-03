## Context

`isolate-completion-pre-dispatch-ingress` уже доставил truthful split для окна `adapter read -> dispatch`, и incident `2026-04-02T15:44:53Z` показывает, что этот слой теперь в основном здоров:

- `adapter_to_dispatch_wait_ms=2-3`;
- `dispatch_to_request_context_wait_ms=0`;
- `turn_wait=0`;
- `service_future_to_first_poll_wait_ms=0-1`.

При этом traces всё ещё имеют два user-visible дефекта:

1. Реальный dominant seam живёт внутри `query_bundle`, а не в ingress.
2. Existing human-readable verdict builder может по-прежнему объявить `adapter_before_dispatch_dominant`, потому что не сравнивает ingress wait с dominant query-body latency.

Есть и третий observability defect: если request заходит в `query_bundle`, а затем завершается `cancelled/superseded`, stage accounting может не записать spent time вообще, и тогда seconds-scale handler tail уходит в `unattributed_overhead`.

## Versioning Note

- Текущий shipped public contract уже на `response.version=19`.
- Предыдущий change зафиксировал contiguous baseline `contracts/lsp-completion-timeline/v16`.
- Этот change целится в непрерывную линию `19 -> 20` и `v16 -> v17`.
- Canonical target-state для этого change задают только `proposal.md`, `design.md`, `tasks.md` и delta spec внутри `openspec/changes/refactor-completion-query-bundle-root-cause-attribution/`.

## Goals / Non-Goals

### Goals

- Сделать query-body latency truthful и bounded в authoritative completion timeline.
- Отделить queue saturation внутри blocking runtime от actual blocking compute.
- Убрать ложный verdict `adapter_before_dispatch_dominant` для traces, где доминирует `query_bundle`.
- Сохранить additive/fail-closed semantics для старых consumers и явную деградацию на `v19`.

### Non-Goals

- Не переписывать заново transport admission и strict-priority ingress scheduler.
- Не обещать hard preemption глубоко внутри `analysis.ir()` в рамках этого change.
- Не маскировать возможный client-side tail как server root cause.
- Не вводить high-cardinality stage names, free-text причины или raw-log dependency.

## Decisions

### 1. Полная миграция consumers идёт на canonical grouped query-body taxonomy

Выбранное решение: `v20` вводит новый canonical grouped vocabulary для query-body stages. Он становится source of truth одновременно для:

- per-request timeline `stages`;
- `dominant_stage` в timeline;
- scale-aware metrics/report consumers;
- human-readable verdict projection;
- incident summary / clipboard export.

Canonical grouped vocabulary:

- `query_bundle_pool_wait`
- `query_bundle_deps_and_file_snapshot`
- `query_bundle_owner_hint`
- `query_bundle_ir_query`
- `query_bundle_ir_retry`
- `query_bundle_other`

Низкоуровневые observability metrics внутри owner-hint path сохраняются, но перестают быть canonical public stage vocabulary. Они MUST нормализоваться к grouped taxonomy при:

- построении `dominant_stage`;
- scale-aware perf reports;
- incident handoff summary;
- derived verdicts.

Legacy aggregate `query_bundle` и `completion_stage_query_bundle_ms` допускаются только как transitional mirror внутри runtime/metrics слоя на период миграции, но:

- MUST NOT быть canonical stage name для `v20`;
- MUST NOT определять `dominant_stage`;
- MUST NOT использоваться acceptance gates или versioned contract baseline `v17`.

Почему так:

- `stages` уже являются authoritative request-level surface;
- extension, clipboard и incident summary уже умеют работать с bounded stage taxonomy;
- grouped vocabulary сохраняет bounded cardinality и не тащит в public surface десятки `query_bundle_owner_hint_*` leaf labels;
- полная миграция consumers убирает двойную canonical truth между aggregate `query_bundle` и new leaf stages.

Следствие:

- `dominant_stage` в `v20` указывает только на grouped `query_bundle*` stage;
- existing metrics/report consumers перепривязываются на grouped taxonomy;
- low-level owner-hint metrics остаются internal drilldown, а не public stage contract.

### 2. Request-level substage attribution выходит из blocking closure через structured carrier

Сейчас query-body computation живёт внутри blocking closure, а request-level `timeline_capture` находится снаружи. Поэтому новый design фиксирует два отдельных carrier objects:

- `ObservedBlockingCall<R>`:
  - `queue_wait`
  - `exec_elapsed`
  - `join_result: Result<R, JoinError>`
- `QueryBundleTraceReport`:
  - bounded `SmallVec<QueryBundleStageSample>` для grouped query-body stages;
  - `covered_exec_ms`;
  - `terminal_status: completed|cancelled|failed`;
  - optional bounded terminal cause (`cancelled_after_retry`, `join_error`, `checkpoint_cancelled`, `handler_error`, `completed`).

Blocking closure возвращает не только business payload, но и `QueryBundleTraceReport`. Наружный async слой:

1. Превращает `queue_wait` в `query_bundle_pool_wait`.
2. Добавляет grouped stages из `QueryBundleTraceReport`.
3. Если `covered_exec_ms < exec_elapsed`, синтезирует `query_bundle_other` на remainder.
4. Если closure упала с `JoinError`, синтезирует `query_bundle_other | failed` на весь `exec_elapsed`.

Это даёт total accounting даже там, где closure не смогла вернуть normal payload.

Сейчас [policy.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/policy.rs) уже считает `queue_wait_elapsed` и `exec_elapsed`, но request trace их не видит. Для root-cause incident analysis этого недостаточно.

Выбранное решение:

- bounded blocking helper возвращает observed queue wait / exec durations вместе с `join_result`;
- closure использует локальный `QueryBundleStageRecorder`/stage guards и собирает `QueryBundleTraceReport`;
- completion handler использует `ObservedBlockingCall + QueryBundleTraceReport` при построении `query_bundle_pool_wait` и downstream grouped stages;
- global metrics сохраняются как отдельный слой, но больше не являются единственным источником этой информации.

### 3. Query-body stage accounting MUST быть total для success/cancel/fail path

Выбранное решение: `query_bundle` instrumentation переводится на stage guard pattern, где stage закрывается в любом terminal path.

Требования:

- если request уже вошёл в query-body path, trace MUST публиковать `query_bundle*` stage и для `cancelled`, и для `failed`;
- leaf stage, завершившийся до cancellation boundary, MAY остаться `completed`; если cancellation происходит после завершения известных leaves, `query_bundle_other` обязан нести cancelled remainder;
- итоговый query-body accounting MUST содержать хотя бы один grouped stage, отражающий terminal `cancelled/failed` tail, если такой tail реально был внутри query-body envelope;
- seconds-scale handler tail после входа в query-body MUST NOT полностью исчезать в `unattributed_overhead`.

Это не означает, что cancellation instantly прерывает `analysis.ir()`. Change обещает truthful accounting и bounded checkpoints, а не hard preemption чужого compute.

### 4. Canonical verdict vocabulary для query-body dominance фиксируется явно

Выбранное решение: extension surfaces используют bounded verdict vocabulary:

- `query_bundle_dominant`
- `query_bundle_pool_wait_dominant`
- `query_bundle_deps_and_file_snapshot_dominant`
- `query_bundle_owner_hint_dominant`
- `query_bundle_ir_query_dominant`
- `query_bundle_ir_retry_dominant`
- `query_bundle_other_dominant`

Правила:

- leaf verdict MAY публиковаться только для grouped `query_bundle*` stage;
- generic `query_bundle_dominant` публикуется вместе с leaf verdict;
- если `dominant_stage` нормализуется из deeper owner-hint metrics, user-facing verdict остаётся `query_bundle_owner_hint_dominant`;
- query-body verdicts имеют precedence над `adapter_before_dispatch_dominant`, `server_before_method_entry_dominant` и `handler_prelude_dominant`.

### 5. Extension verdict builder должен перестать быть ingress-only эвристикой

Выбранное решение:

- host-side verdict logic становится единственным source of truth;
- webview перестаёт дублировать собственную эвристику или использует тот же helper;
- `adapter_before_dispatch_dominant` публикуется только если ingress wait реально доминирует над later-stage latency и query-body dominance не доказана;
- если authoritative `dominant_stage` или stage durations показывают `query_bundle*` dominance, derived verdict обязан следовать grouped query-body vocabulary, а не спорить с ним.

### 6. `v19` degradation остаётся явной и fail-closed

Новый truthful query-body split появляется только на `v20`.

Для `v19` consumers:

- не выдумывают grouped `query_bundle_pool_wait`, `query_bundle_owner_hint` или `query_bundle_ir_query`;
- могут показывать legacy aggregate `query_bundle`, если он есть;
- обязаны явно отмечать, что detailed query-body split unavailable by design.

## Alternatives Considered

### A. Чинить только extension verdict logic

Отклонено. Это уберёт ложный `adapter_before_dispatch_dominant`, но оставит blind spot на cancelled query path и не позволит отделить pool saturation от actual blocking exec.

### B. Добавить только новые runtime метрики

Отклонено. Global metrics не дают request-bound evidence для конкретного incident trace.

### C. Делать hard cancellation внутри `analysis.ir()` частью этого change

Отклонено как слишком большой scope. Это возможный follow-up, если после truthful split останется неприемлемый superseded tail.

## Risks / Trade-offs

- Разбиение `query_bundle` может изменить привычную интерпретацию `dominant_stage`.
  - Mitigation: contiguous version bump `19 -> 20`, explicit degradation на `v19`, обновлённый baseline.
- Cancelled trace теперь покажет более неприятную правду о seconds-scale query-body tail.
  - Mitigation: это ожидаемый результат, а не regression; acceptance должен проверять truthful accounting, а не косметическое скрытие.
- Host/webview deduplication verdict logic может затронуть clipboard и incident summary одновременно.
  - Mitigation: держать один bounded verdict source и покрыть его focused extension tests.
