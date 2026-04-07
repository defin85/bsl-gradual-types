## Context

Новый diagnostics save timeline после `add-04`, `refactor-05` и `refactor-06` уже даёт truthful
картину:

- `save_fastlane` first publish после `didSave` стал bounded;
- `save_cycle_sequence` отделил operator-facing ordering от общего `diagnostics_generation`.

Однако bundle `2026-04-07T20:23:03Z` показывает, что remaining save-tail сидит в другом месте:

- `wait_for_file_version_diagnostics_ms.p95=6924`;
- `runtime_wait_for_file_version_queue_wait_ms.p95=9837`;
- `runtime_apply_change_set_file_exec_ms.p95=7305`;
- один save cycle остаётся `idle_heavy_outcome=pending`, другой уходит в
  `idle_heavy_outcome=superseded_generation`.

Следовательно, fast first refresh уже исправлен, но eventual heavy follow-up всё ещё может
проигрывать гонку writer/apply path и не давать пользователю richer same-version diagnostics вовремя.

## Goals

- Уменьшить или обойти apply-lag как primary gate для `didSave` heavy follow-up.
- Сохранить same-version correctness и supersession semantics.
- Сделать root cause follow-up stall различимым в request-centric diagnostics save timeline.

## Non-Goals

- Не возвращать старый unbounded wait в `save_fastlane`.
- Не строить full per-`didChange` request trace.
- Не переписывать весь diagnostics scheduler или writer thread.

## Decisions

### 1. didSave heavy follow-up должен предпочитать same-version ready artifacts

Если `save_fastlane` уже дал same-version first publish, `idle_heavy` не должен повторно
рассматривать `wait_for_file_version` как primary gate по умолчанию.

Follow-up path должен предпочитать:

1. same-version applied semantic state, если он уже готов;
2. same-version ready parse snapshot + lightweight semantic preparation;
3. только затем bounded wait/rehandoff path к writer-owned analysis state.

Идея не в том, чтобы убрать writer truth, а в том, чтобы не блокироваться на нём первой же
операцией, когда save already materialized same-version artifacts.

### 2. Follow-up stall attribution должен быть request-centric

Текущий diagnostics save timeline уже показывает first publish truthfully, но pending heavy follow-up
не объясняет, stalled ли он на apply-lag, semantic query или supersession.

Нужен low-cardinality follow-up attribution, например:

- `followup_wait_reason=apply_lag|semantic_work|pending_publish|superseded`;
- optional bounded `followup_wait_for_file_version_ms` / `followup_snapshot_with_deps_ms`.

Это должно оставаться request-centric и server-authored, без реконструкции из aggregate metrics.

### 3. Monotonic improvement contract

После `save_fastlane` пользователь должен либо быстро получить richer heavy publish того же
`save_cycle_sequence`, либо diagnostics save timeline должен truthful показать, что cycle застрял на
apply-lag/supersession. “Просто pending без причины” для operator workflow больше недостаточно.

## Alternatives Considered

### Только ускорить `apply_change(SetFile)`

Полезно, но недостаточно. Это может снизить p95, но не гарантирует, что heavy follow-up перестанет
быть hostage у writer lag.

### Оставить только cumulative metrics

Недостаточно для incident bundle. Metrics уже показывают large wait, но не связывают его с
конкретным save cycle.

### Убрать `idle_heavy` после save вообще

Неприемлемо. Это сломает richer/final diagnostics contract и оставит пользователя только с
syntax-only publish.

## Risks / Trade-offs

- Более агрессивный same-version follow-up path может начать дублировать куски semantic pipeline.
  Это нужно держать узким и использовать только на `didSave`.
- Добавление follow-up wait attribution поднимет contract version diagnostics save timeline ещё раз.
- Если bounded follow-up path выбрать слишком узким, можно улучшить latency, но потерять слишком
  много richer diagnostics. Нужен acceptance на representative `conf_big`.

## Validation

1. Regression: delayed `apply_change(SetFile)` больше не держит `idle_heavy` follow-up в long
   pending, если same-version ready artifacts уже есть.
2. Regression: diagnostics save timeline объясняет pending heavy follow-up через explicit
   request-centric wait reason, а не через пустой `pending`.
3. Live report: representative `conf_big didSave` scenario показывает bounded fastlane и
   materially improved heavy follow-up readiness или truthful apply-lag attribution.
