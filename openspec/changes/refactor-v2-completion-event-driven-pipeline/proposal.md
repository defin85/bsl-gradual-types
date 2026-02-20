# Change: Полная event-driven rearchitecture очередей интерактивного completion v2

## Why
Текущий runtime-centric трек (`improve-v2-completion-interactive-reliability`) закрывает критичные UX-проблемы без большого риска, но не устраняет архитектурный первопричинный класс проблем: конкуренцию `didChange`/completion, сложную деградацию при transient-cancel и ограниченную предсказуемость latency при burst-нагрузке.

Для долгосрочной устойчивости интерактивного пути нужен отдельный архитектурный шаг: перейти на event-driven orchestration очередей и жизненного цикла интерактивных запросов.

### Observed Baseline (2026-02-20, conf_big, warm run)
- `completion_duration_ms`: p50=3246ms, p95=3685ms, p99=3685ms (n=7)
- `intellisense_v2_snapshot_completion_ms`: p50=2773ms, p95=3623ms, p99=3623ms (n=7)
- `intellisense_v2_wait_for_file_version_completion_ms`: p50=2ms, p95=121ms, p99=121ms (n=7)
- `intellisense_v2_runtime_queue_wait_background_ms`: p50=0ms, p95=2788ms, p99=3303ms (n=123)
- `intellisense_v2_syntax_diagnostics_query_ms`: p50=2570ms, p95=2858ms, p99=2983ms (n=21)
- `intellisense_v2_semantic_diagnostics_query_ms`: p50=0ms, p95=584ms, p99=861ms (n=21)
- `intellisense_v2_interactive_wait_budget_exhausted_total`: 2

Эти данные показывают, что проблема лежит не в одном query-этапе, а в orchestration модели: горячий completion путь и фоновые стадии конкурируют за ресурсы, а snapshot+queue-wait дают тяжелый tail.

## Target Architecture
Целевая архитектура: per-file event-driven orchestrator (actor) на границе LSP/runtime с bounded очередью, deterministic ordering, latest-wins scheduling и явной cancellation propagation.

Ключевые свойства целевой архитектуры:
- устраняет adapter-local процедурную связность hot path;
- дает формальный контракт порядка/отмены для burst `didChange`/completion;
- поддерживает безопасный dual-mode rollout (`legacy` + `event-driven`) без breaking change клиентского контракта.

### Architecture Lock
Для данного change целевая архитектура жёстко фиксируется как per-file event-driven orchestrator.
Любые отклонения от этой архитектуры находятся вне scope данного change.

## What Changes
- **ADDED**: requirement в `bsl-intellisense-v2` про event-driven orchestration интерактивного completion pipeline через per-file dispatcher/actor и bounded очередь событий.
  - `didChange` ingest MUST оставаться неблокирующим для интерактивного completion;
  - `didChange`/completion/cancel MUST обрабатываться как формализованные события.
- **ADDED**: requirement в `bsl-intellisense-v2` про deterministic ordering + latest-wins semantics.
  - completion для устаревшей ревизии MUST коалесцироваться/отменяться до тяжелых стадий;
  - актуальный запрос MUST получать приоритет в интерактивном бюджете.
- **ADDED**: requirement в `bsl-intellisense-v2` про cancellation contract.
  - LSP cancel MUST доходить до orchestrator и прерывать дальнейшие тяжелые стадии между checkpoint-этапами.
- **ADDED**: requirement в `bsl-intellisense-v2` про rollout/rollback контракт.
  - режим MUST управляться feature-flag mode (`off|shadow|canary|on`);
  - MUST быть безопасный kill-switch rollback к legacy/runtime-centric пути.
- **ADDED**: requirement в `bsl-intellisense-v2` про observability сравнение legacy vs event-driven.
  - MUST быть mode-aware разрез метрик для формального pass/fail rollout-гейтов.

## Acceptance Contract (Implementation-Ready)
- Пользовательский LSP-контракт completion MUST оставаться backward-compatible во всех режимах (`off|shadow|canary|on`):
  - без изменений формы `CompletionResponse`/`CompletionItem`;
  - без требования ручных изменений editor settings у пользователя.
- `off` MUST быть безопасным default-режимом запуска.
- Rollout переходы MUST быть gated:
  - `off -> shadow`: разрешается только после прохождения контрактных тестов event-order/cancel/backpressure;
  - `shadow -> canary`: разрешается только после pass mode-aware SLO-гейтов на warm-профиле;
  - `canary -> on`: разрешается только после стабильного pass на canary-профиле и проверки parity drift.
- Kill-switch rollback MUST быть мгновенным переключением mode в `off` без перезапуска процесса.

### Rollout Pass/Fail Gates
- `completion_duration_ms` p95 `<= 1500ms` (warm profile).
- `intellisense_v2_wait_for_file_version_completion_ms` p95 `<= interactive_wait_budget_ms + 20ms`.
- `intellisense_v2_runtime_queue_wait_interactive_ms` p95 `<= interactive_wait_budget_ms + 250ms`.
- `completion_cancelled_rate <= 0.10`.
- `completion_parity_drift_rate <= 0.01` (для `shadow`/`canary`).
- `member_access_terminal_empty_missing_ir_rate <= 0.005` (для `shadow`/`canary`).

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/runtime_config.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `backend/src/bin/lsp_server/server/core.rs` (контрактные и нагрузочные тесты)
  - `backend/tests/lsp_incremental_completion_test.rs`

## Dependencies
- Реализуется отдельным треком после стабилизации `improve-v2-completion-interactive-reliability`, чтобы не блокировать быстрые UX-исправления.

## Baseline Reference
- Baseline для acceptance фиксируется датированным артефактом с обязательными полями: `profile`, `mode`, `n`, `p50/p95/p99`, `pass/fail`.
- Источник baseline для текущего change: наблюдения `2026-02-20` в этом proposal и профильные тесты warm-path в `backend/src/bin/lsp_server/server/core.rs`.

## Scope
- В scope: orchestration/очереди/политика отмены/коалесцирование событий/гарантии порядка/наблюдаемость/rollout.
- Вне scope: новые completion features (новые кандидаты, ranking-модель, расширение типового покрытия).
