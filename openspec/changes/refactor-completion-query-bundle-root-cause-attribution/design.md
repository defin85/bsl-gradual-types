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

### 1. Truthful query-body attribution идёт через bounded stage taxonomy, а не через свободный текст

Выбранное решение: `v20` остаётся stage-centric contract. Вместо opaque aggregate `query_bundle` timeline MAY раскладывать query-body path на bounded stage names:

- `query_bundle_pool_wait`
- `query_bundle_deps_and_file_snapshot`
- `query_bundle_ir_query`
- `query_bundle_ir_retry`
- `query_bundle_other`

Почему так:

- `stages` уже являются authoritative request-level surface;
- extension, clipboard и incident summary уже умеют работать с bounded stage taxonomy;
- это позволяет сохранить additive evolution без отдельного raw-only debug object.

Следствие:

- `dominant_stage` в `v20` может указывать на конкретный `query_bundle*` stage;
- aggregate `query_bundle` не обязан сохраняться, если trace публикует breakdown stage set.

### 2. Pool wait и blocking exec фиксируются в request-local trace, а не только в глобальных метриках

Сейчас [policy.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/policy.rs) уже считает `queue_wait_elapsed` и `exec_elapsed`, но request trace их не видит. Для root-cause incident analysis этого недостаточно.

Выбранное решение:

- bounded blocking helper возвращает observed queue wait / exec durations вместе с result;
- completion handler использует эти значения при построении `query_bundle_pool_wait` и downstream query stages;
- global metrics сохраняются как отдельный слой, но больше не являются единственным источником этой информации.

### 3. Query-body stage accounting MUST быть total для success/cancel/fail path

Выбранное решение: `query_bundle` instrumentation переводится на stage guard pattern, где stage закрывается в любом terminal path.

Требования:

- если request уже вошёл в query-body path, trace MUST публиковать `query_bundle*` stage и для `cancelled`, и для `failed`;
- stage status обязан совпадать с terminal outcome path (`completed|cancelled|failed`);
- seconds-scale handler tail после входа в query-body MUST NOT полностью исчезать в `unattributed_overhead`.

Это не означает, что cancellation instantly прерывает `analysis.ir()`. Change обещает truthful accounting и bounded checkpoints, а не hard preemption чужого compute.

### 4. Extension verdict builder должен перестать быть ingress-only эвристикой

Выбранное решение:

- host-side verdict logic становится единственным source of truth;
- webview перестаёт дублировать собственную эвристику или использует тот же helper;
- `adapter_before_dispatch_dominant` публикуется только если ingress wait реально доминирует над later-stage latency;
- если authoritative `dominant_stage` или stage durations показывают `query_bundle*` dominance, derived verdict обязан следовать им, а не спорить с ними.

### 5. `v19` degradation остаётся явной и fail-closed

Новый truthful query-body split появляется только на `v20`.

Для `v19` consumers:

- не выдумывают `query_bundle_pool_wait` или `query_bundle_ir_query`;
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
