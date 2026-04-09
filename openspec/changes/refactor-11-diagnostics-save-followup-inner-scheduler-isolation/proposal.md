# Change: reserve inner scheduler capacity for didSave follow-up isolation

## Why
Свежий incident bundle `2026-04-09T06-51-37Z` показал, что `refactor-10` закрыл outer admission gap и additive telemetry, но не устранил сам residual tail. Для `didSave` follow-up на representative `conf_big`:

- `save_fastlane` first publish остаётся bounded (`40ms`);
- `idle_heavy` доходит только через `18715ms`;
- dominant blocker уже не UI и не publish, а `followup_runtime_queue_wait_ms=15186`, при вторичных `apply_lag_ms=463` и `semantic_diagnostics_query_ms=3464`.

Code audit подтвердил причину: `AdmissionLane::DidSaveFollowup` сейчас протаскивается как observability label, но не как реальный scheduler input внутри writer/runtime и blocking CPU admission. Admitted follow-up всё ещё может:

- попасть в общий `Background` writer backlog;
- заново ждать generic `Background` CPU permit;
- конкурировать с unrelated auxiliary/background work вроде `bsl.getCurrentContext`.

Это оставляет loophole в уже утверждённом intent `refactor-10`: outer lane существует, но admitted follow-up всё ещё наследует generic background contention after admission.

## What Changes
- Зафиксировать в `bsl-intellisense-v2`, что didSave follow-up slot не считается fully admitted, пока он не владеет inner-scheduler execution entitlement, вырезанным из существующего bounded non-interactive budget.
- Потребовать lane-aware arbitration внутри existing writer/runtime scheduler и blocking CPU admission, чтобы admitted `did_save_followup` не возвращался в generic background wait paths.
- Сохранить existing outer owner в diagnostics runtime, binary `CpuWorkClass`, current runtime-config knob, `disabled_by_config` outcome и additive lane telemetry.
- Добавить deterministic regressions и representative live evidence, которые проверяют именно inner-scheduler isolation, а не только наличие outer lane и метрик.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`

## Non-Goals
- Не вводить новый runtime-config key или менять zero/default semantics текущего quota knob.
- Не повышать didSave follow-up до `Interactive` и не добавлять третий `CpuWorkClass`.
- Не перепроектировать весь writer/runtime scheduler для всех background jobs.
- Не лечить process-wide writer/apply outliers beyond request-centric follow-up isolation.
