# Change: eliminate redundant semantic parse in didSave follow-up

## Why

Свежий incident bundle `2026-04-11T16-14-14Z` показывает, что после `refactor-10`
`save_fastlane` first publish уже bounded (`43-53ms`), но `didSave + idle_heavy`
follow-up всё ещё публикуется за `3629ms` и `9293ms`.

Request-centric trace уже локализует tail точнее, чем раньше:

- `runtime_queue_wait_ms=538` и `5931`;
- `semantic_diagnostics_query_ms=3011` и `3301`;
- `apply_lag_ms=680` и `466`.

При этом metrics snapshot показывает, что внутри самой semantic diagnostics стадии
доминирует `parse_result_ms` (`p95=3011ms`), хотя save-flow уже имеет same-version
syntax reuse и same-version parse-backed state.

Read-only расследование кода подтверждает root cause:

- `AnalysisV2::parse_result()` и `AnalysisV2::ir_profiled()` уже умеют работать
  snapshot-aware;
- но `semantic_diagnostics_profiled()` и flow-sensitive вариант обходят эти accessors и
  бьют напрямую в salsa `parse_result(...)` / `ir(...)`;
- `didSave` follow-up сначала пытается `shadow_state` semantic path и только потом
  `ready_artifacts`, поэтому same-version ready parse snapshot underused even when already ready.

Иными словами, прошлые changes закрыли syntax reuse и background isolation, но semantic stage
ещё не использует тот же parse-backed reuse contract.

## What Changes

- Зафиксировать в `bsl-intellisense-v2`, что same-version `didSave + idle_heavy` follow-up
  MUST предпочитать уже готовые `ready_artifacts` немедленно, если same-version ready parse
  snapshot уже materialized к моменту старта follow-up.
- Потребовать, чтобы snapshot-backed semantic diagnostics profile helpers использовали
  snapshot-aware `parse_result` / `ir_profiled` accessors и не форсировали redundant direct salsa
  parse/IR recompute для `SetFileWithSnapshot`-backed analysis.
- Сохранить fail-closed fallback: при отсутствии или mismatch same-version snapshot система
  MUST оставаться на truthful shadow/generic path и не публиковать stale diagnostics.
- Расширить request-centric didSave save timeline bounded source attribution для semantic stage,
  чтобы оператор видел, какой follow-up path использовался и откуда пришли parse/IR inputs.
- Поднять versioned diagnostics save timeline contract (`bsl.getDiagnosticsSaveTimeline`)
  additive `v7 -> v8`, чтобы bounded semantic path/source attribution имел explicit consumer
  degradation semantics вместо silent omission на older payloads.
- Добавить детерминированные regressions и representative live evidence на `conf_big`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/lib/analysis_api.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - incident bundle / diagnostics save timeline projections under `backend/` and `vscode-extension/`

## Non-Goals

- Не менять `save_fastlane` semantics или first-publish budget.
- Не перепроектировать dedicated `did_save_followup` lane/quota model из `refactor-10`.
- Не лечить весь residual `runtime_queue_wait_ms`; этот change бьёт в redundant semantic
  parse/IR cost и path selection, а не в каждую background queue outlier.
