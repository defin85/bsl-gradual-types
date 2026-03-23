# Change: fast-lane current-revision readiness after document-sync handoff

## Why
После реализации short-lived document-sync path пользовательский отклик заметно улучшился, но incident bundle `2026-03-23T08:03:23Z` показывает, что residual tail сместился за transport boundary, а не исчез полностью:
- по сравнению с bundle `2026-03-22T16:19:59Z` медиана `service_future_to_first_poll_wait_ms` упала с `5857ms` до `80ms`, то есть transport admission действительно улучшился;
- при этом request `50` имеет `service_future_to_first_poll_wait_ms=1ms`, но всё равно умирает как `prepare_timeout@prepare_guard` после `3030ms` в `wait_for_file_version`;
- request `57` уже видит `observed_file_version=9`, но завершает `exact_deadline` с `head_ready=false` и `exact_ready=false`;
- cumulative metrics показывают `intellisense_v2_runtime_wait_for_file_version_queue_wait_ms p99=9698ms` и `intellisense_v2_runtime_type_index_precompute_exec_ms p50=3485ms`, тогда как `intellisense_v2_runtime_queue_wait_interactive_ms p95=5ms`.

Это означает, что главный остаточный bottleneck теперь находится не в LSP transport slots, а в current-revision apply/head readiness path, который продолжает конкурировать с slow background work.

## What Changes
- Добавить в `bsl-intellisense-v2` контракт fast-lane для current-revision readiness после `didOpen/didChange` handoff:
  - same-file продвижение `applied_version` получает interactive-priority;
  - current-revision `CompletionHeadArtifact` публикуется и становится queryable независимо от exact/type-index/deferred-diagnostics readiness;
  - latest-wins и supersession semantics сохраняются.
- Уточнить churn-aware completion contract:
  - `prepare_timeout` на фазе `wait_for_file_version` после same-file handoff считается регрессией readiness scheduler, а не допустимым bounded fail-closed исходом;
  - `exact_deadline` при уже достигнутом current `observed_file_version`, но `head_ready=false`, считается регрессией head-readiness fast lane, а не приемлемой exact-upgrade latency.
- Расширить representative real-module gate отдельным post-handoff readiness профилем через live LSP path:
  - gate использует уже существующие authoritative поля `wait_for_file_version_runtime`, `min_file_version`, `observed_file_version`, `head_ready_before_wait`, `artifact_poll`;
  - gate получает численные pass/fail budgets для queue wait outliers;
  - gate валится на `prepare_timeout@wait_for_file_version` и на post-apply `head_ready=false` gap.
- Сохранить strict current-revision / fail-closed semantics без stale fallback и без подмены current revision ранее построенным semantic payload.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - representative perf / acceptance harness around real-module completion gate

## Non-Goals
- Возвращать stale или partial completion как substitute для current revision.
- Оптимизировать full exact/type-index throughput вне контекста first current-revision response.
- Повторно открывать scope transport-slot retention из `refactor-lsp-document-sync-slot-release`.
- Расширять observability contract новыми high-cardinality debug fields вместо использования уже существующих bounded полей.
