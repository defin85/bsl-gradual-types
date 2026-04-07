## Context

После `refactor-03`, `add-04` и `refactor-05` bundle уже truthfully показывает `didSave`
refresh. Новый live capture `2026-04-07T19:16:12Z` сузил remaining bottleneck:

- `diagnostics-save-trace-15.first_publish.elapsed_ms=6651`;
- `blocking_queue_wait_ms=6592`;
- `syntax_diagnostics_query_ms=35`.

Это значит, что hot path first publish стоит не на syntax query, а на shared bounded blocking
queue перед shadow parse fallback.

Тот же bundle выявил operator-facing anomaly: для одного `requested_version` timeline может
показывать traces с confusing order по `diagnostics_generation`. Это expected на уровне runtime,
потому что `diagnostics_generation` общий для всех diagnostics triggers, но как save-cycle identity
он неточен.

## Goals

- Убрать seconds-scale starvation у `save_fastlane` syntax-only first publish.
- Сохранить same-version truthful first publish.
- Дать dedicated monotonic save-cycle identity, понятный оператору и bundle summary.
- Не ломать existing supersession logic, которая всё ещё использует `diagnostics_generation`.

## Non-Goals

- Не переписывать весь diagnostics scheduler.
- Не делать per-`didChange` timeline.
- Не удалять `diagnostics_generation` из trace; он остаётся полезным low-level fact.

## Decisions

### 1. save_fastlane shadow fallback bypass-ит shared interactive budget

Когда `save_fastlane` не может взять diagnostics из applied-analysis или ready parse snapshot и
падает в shadow parse fallback, он не должен ждать shared bounded interactive permit.

Вместо этого fallback:

- идёт через dedicated blocking path без shared `CpuBoundBudget`;
- сохраняет syntax-only semantics и same-version truthfulness;
- остаётся observability-visible через existing `syntax_diagnostics_query_ms`.

`blocking_queue_wait_ms` для этого path MUST больше не отражать shared-budget wait. Если
shared-budget wait отсутствовал, поле может быть `0` или omitted.

### 2. Save timeline получает dedicated `save_cycle_sequence`

На `didSave` сервер выделяет отдельный monotonically increasing `save_cycle_sequence` per file.

`save_cycle_sequence`:

- инкрементируется только на `didSave`;
- проходит через весь diagnostics save lifecycle;
- участвует в save timeline identity и operator-facing ordering;
- не заменяет `diagnostics_generation` в cancellation/supersession logic.

### 3. Save-cycle correlation опирается на sequence, не на generation

Timeline и incident summary должны явно показывать:

- `requested_version`;
- `save_cycle_sequence`;
- `diagnostics_generation`.

Для одинакового `requested_version` операторский порядок MUST определяться `save_cycle_sequence`, а
не случайным сочетанием `started_at_ms` и `diagnostics_generation`.

## Risks / Trade-offs

- Bypass shared queue может дать дополнительную blocking parallelism под burst save load. Это
  допустимо, потому что path узкий: только `didSave`, только syntax-only fallback, только один файл.
- Появляется ещё один per-file counter. Это маленькая bounded state map, которую нужно чистить на
  `didClose`.
- Старые clients не знают про `save_cycle_sequence`; поэтому contract version diagnostics save
  timeline нужно повысить совместно в backend и extension.

## Validation

1. Regression: interactive queue saturation больше не задерживает `didSave save_fastlane` first
   publish на секунды только из-за shared budget.
2. Regression: два `didSave` для одного `requested_version` имеют monotonic `save_cycle_sequence`
   и не требуют operator ordering по `diagnostics_generation`.
3. Bundle/summary tests: save diagnostics section рендерит `save_cycle_sequence` и не подразумевает
   save-cycle ordering через `diagnostics_generation`.
