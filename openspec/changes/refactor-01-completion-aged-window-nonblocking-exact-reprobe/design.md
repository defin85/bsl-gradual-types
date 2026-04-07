## Контекст

Incident bundle `2026-04-07T09:36:20Z` показал path, где:

- `prepare_apply_age_at_start_ms=308`;
- response остаётся semantic/non-empty;
- total request latency уходит в `1590ms`;
- visible stages заканчиваются на `154ms`.

По коду это совпадает с aged non-member веткой в
`backend/src/bin/lsp_server/server/language_server/impl_completion.rs`, где request path до
`query_bundle_started` синхронно вызывает
`completion_current_revision_snapshot_for_origin_and_operation(...).await`.

В runtime facade этот вызов может сделать несколько `snapshot_with_priority()` попыток и затем
упасть в `snapshot_with_deps_with_priority(...)`, так что wait может быть длинным и при этом не
виден в stage taxonomy.

## Цели

- Убрать blocking exact re-probe из aged non-member first-response path.
- Сохранить truthful current-revision semantics без возврата stale semantic substitute.
- Сделать timeline truthful для residual blocking work и убрать seconds-scale uncovered handler gap.

## Не-цели

- Не менять member-access exact wait contract.
- Не решать latest-only/supersession policy для `bsl.getCurrentContext` в этом change.
- Не перепроектировать completion transport или client-side Observability UI.

## Решения

### 1. Aged non-member first response использует уже подготовленный current-revision state

Если request идёт через `shadow_current_revision_fast_path`, не является member-access и вышел из
immediate apply-age window, handler не должен синхронно re-probe-ить свежий current-revision
snapshot только ради того, чтобы проверить exact ещё раз перед first response.

Допустимы только два варианта:

- exact уже доказан как ready из подготовленного состояния, и тогда request может отдать exact;
- exact не доказан, и тогда request сразу идёт в bounded lightweight/no-IR path.

### 2. Exact warmup остаётся вне critical path first response

Background exact upgrade или ранее запланированная current-revision work может продолжаться, но aged
invoked completion не должен ждать её синхронно перед terminal decision first response.

Это сохраняет текущую идею v2 contract:

- first response строится из truthful current-revision artifacts;
- exact enrichment не обязателен как prereq для bounded current-revision response;
- stale exact другой revision всё так же запрещён.

### 3. Timeline не должен скрывать current-revision snapshot reacquisition

Если implementation всё ещё делает blocking current-revision snapshot reacquisition до terminal
first-response decision, эта работа должна появиться как отдельный low-cardinality stage в
authoritative timeline.

Representative aged-path validation дополнительно должна fail-ить, если `total_duration_ms`
существенно больше последнего видимого stage end и gap объясняется не bounded capture overhead, а
скрытой request-path работой.

### 4. Acceptance должен воспроизводить реальный incident profile

Guard нужен не на synthetic cold miss, а на realistic same-file invoked completion:

- large-module fixture;
- non-member current-revision path;
- apply-age уже вышел из immediate window;
- first response остаётся bounded и truthful без blocking exact re-probe.

## Alternatives Considered

### Добавить только observability без изменения request path

Недостаточно. Timeline станет честнее, но user-visible latency останется.

### Оставить blocking re-probe, но уменьшить budget

Это превращает hidden wait в более ранний fail-closed outcome, а не убирает причину.

### Всегда требовать exact после immediate window

Это возвращает effectively exact-only поведение и ломает current-revision first-response intent.

## Риски и trade-offs

### Риск: aged non-member response станет беднее до готовности exact

Это допустимо, если response остаётся truthful current-revision и bounded. Более богатый exact может
достраиваться позже.

### Риск: stage taxonomy расползётся

Нужен один low-cardinality stage для snapshot reacquisition, а не динамические stage names.

### Риск: background exact warmup начнёт делать лишнюю работу

Validation должна следить, чтобы remediation не превращалась в скрытый expensive prewarm.

## Migration / Rollout

1. Зафиксировать новый step-1 contract в OpenSpec.
2. Перевести aged non-member path на non-blocking first response.
3. Добавить truthful coverage gate и incident-like evidence.
