# LSP v2 Latency Policy

## Purpose

Этот документ фиксирует рабочую политику latency/freshness для `IntelliSense v2` в LSP после fail-closed cutover и связанные runtime knobs/метрики.

## Operation Classes

Runtime queue priority для LSP v2 сейчас фиксирован так:

- `interactive`: `completion`, `hover`, `signatureHelp`, `definition`, `members`,
  `type_at_position`
- `background`: `diagnostics` и остальные non-priority операции (`document_symbol`,
  `references`, `rename`, `symbol_search`)

## Interactive Freshness Policy

Для `interactive` операций используется bounded wait + fail-closed завершение:

1. Сначала выполняется ожидание `min_file_version`.
2. Ожидание ограничено `intellisense_v2_interactive_wait_budget_ms` (default `120ms`, clamp `[10, 2000]`).
3. Если бюджет ожидания исчерпан или exact current-revision semantic artifacts ещё недоступны, операция завершается fail-closed для текущей revision.
4. Default runtime path не публикует stale/degraded/search-backed semantic substitute и не маскирует semantic truth другой revision под current response.

`members` и `type_at_position` в shared runtime тоже читают exact current-revision canonical
artifact и теперь используют ту же interactive-priority runtime queue, что и остальные
user-facing точечные semantic queries.

## Diagnostics Freshness Policy

`diagnostics` остаётся strict-latest:

- publish происходит только для актуальных `file_version + deps_id + settings_id`;
- stale результаты не публикуются и не могут перезаписать более новую ревизию.
- speculative `parse_result` prefetch в diagnostics-path отключён (уменьшение parse-stage contention);
- diagnostics строится через `syntax_diagnostics` + `semantic_diagnostics` без обязательного отдельного parse prefetch.

## Singleflight Policy

Для дорогих revision-bound query используется singleflight по каноническому ключу:

- `parse_result` / `syntax_diagnostics`: `(file_id, file_version, file_signature, query_kind)`
- `ir`: `(file_id, file_version, file_signature, deps_id, settings_id, query_kind)`

`query_kind`:

- `parse_result`
- `syntax_diagnostics`
- `ir`

Правила:

- один leader на ключ;
- followers получают терминальный outcome leader;
- внутри текущего flight нет auto-retry при error/cancel;
- in-flight запись очищается после завершения leader.

## CPU Budgeting Policy

Для blocking CPU-path используются классы budget:

- минимум 1 permit зарезервирован под `interactive`;
- при `total_permits >= 4` под `interactive` резервируется 2 permits;
- минимум 1 permit зарезервирован под `background` (если total permits >= 2);
- при пустой очереди противоположного класса разрешён borrow;
- `background` не забирает `shared` permit, пока в очереди есть `interactive` waiters;
- при конкуренции fairness восстанавливается.

## Cancellation Checkpoints

В тяжёлых query-пайплайнах `analysis-v2` добавлены ранние checkpoints через
`db.unwind_if_revision_cancelled()`:

- `parse_result`
- `ir`
- `syntax_diagnostics`
- `semantic_diagnostics`
- `type_index`
- `semantic_diagnostics_flow_sensitive`

Это снижает лишнюю CPU-нагрузку после отменённых запросов и ускоряет освобождение budget для новых интерактивных операций.

## Runtime Knobs

- `BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS`

Legacy config surface:

- `BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_VERSION_GAP`
- `BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_AGE_MS`

Эти knobs остаются compatibility/config surface, но не включают stale semantic fallback на default path.

## Observability Contract (required keys)

Counters:

- `intellisense_v2_completion_result_total_{ok_non_empty|ok_empty|fail_closed|cancelled|handler_error}`
- `intellisense_v2_fail_closed_reason_total_origin_<origin>_operation_<operation>_reason_<reason>`
- `intellisense_v2_interactive_wait_budget_exhausted_total`
- `intellisense_v2_interactive_stale_served_total`
- `intellisense_v2_completion_stale_fallback_total`
- `intellisense_v2_interactive_knob_clamped_total`
- `intellisense_v2_singleflight_leader_total`
- `intellisense_v2_singleflight_shared_total`
- `intellisense_v2_runtime_queue_wait_interactive_total`
- `intellisense_v2_runtime_queue_wait_background_total`
- `intellisense_v2_runtime_exec_interactive_total`
- `intellisense_v2_runtime_exec_background_total`

Histograms:

- `intellisense_v2_singleflight_wait_ms`
- `intellisense_v2_runtime_queue_wait_interactive_ms`
- `intellisense_v2_runtime_queue_wait_background_ms`
- `intellisense_v2_runtime_exec_interactive_ms`
- `intellisense_v2_runtime_exec_background_ms`

Bounded fail-closed taxonomy:

- reasons: `missing_canonical_ir`, `missing_semantic_index`, `superseded_revision`, `cancelled`, `unavailable_by_contract`
- origins: `lsp`, `web`, `agent`, `runtime`
- operations in public metric labels use snake_case suffixes, for example:
  - `completion`
  - `hover`
  - `definition`
  - `signature_help`
  - `members`
  - `type_at_position`

Rates:

- `intellisense_v2_parse_result_singleflight_shared_rate`
- `intellisense_v2_parse_result_query_cancel_rate`
- `completion_incomplete_rate`
- `completion_error_rate`

Legacy stale counters остаются guardrail-метриками и на authoritative fixtures должны оставаться нулевыми:

- `intellisense_v2_interactive_stale_served_total == 0`
- `intellisense_v2_completion_stale_fallback_total == 0`

Legacy/internal note:

- старые completion-only assets в репозитории всё ещё могут содержать `terminal_empty_missing_ir_rate`
  или `intellisense_v2_completion_fallback_unavailable_total` как исторические/internal сигналы;
  authoritative public baseline для change больше не использует их как bounded taxonomy.

## Alerting Baseline (warm-path)

Рекомендуемые первичные алерты/пороги:

- `intellisense_v2_observability_contract_violation_total > 0` за интервал прогона.
- `intellisense_v2_interactive_stale_served_total > 0` или `intellisense_v2_completion_stale_fallback_total > 0` на authoritative run.
- рост `intellisense_v2_fail_closed_reason_total_origin_<origin>_operation_<operation>_reason_missing_semantic_index`
  на representative fixtures: это fail-closed availability signal, но не повод возвращать stale/degraded rescue.
- `intellisense_v2_parse_result_singleflight_shared_rate < 0.15` (низкая эффективность singleflight).
- `intellisense_v2_parse_result_query_cancel_rate > 0.20` (чрезмерные cancellations в parse-stage).
- `intellisense_v2_runtime_queue_wait_interactive_ms.p95 > 500ms` (interactive starvation risk).
- `completion_incomplete_rate > 0.30` или `completion_error_rate > 0.05`.

## Representative Perf Gate

Authoritative checked-in perf gate для cutover acceptance теперь строится через
`contracts/intellisense-perf-gate/v2/` и `scripts/run-intellisense-perf.sh`.

Representative matrix:

- operations: `completion`, `hover`, `definition`, `type_at_position`, `members`
- fixture families: `steady_member_chain`, `post_did_change_current_revision`,
  `object_module_explicit_context`, `recordset_module_explicit_context`,
  `incomplete_syntax_member_access`

Cutover perf evidence считается valid только если:

- report покрывает весь representative matrix;
- `coverage.authoritative_for_cutover_acceptance = true`;
- anti-rescue counters (`stale_fallback_total`, `stale_served_total`,
  `degraded_substitute_total`, `search_backed_substitute_total`) остаются нулевыми;
- baseline/provenance проходят blocking verification;
- ratio-based regression checks используют checked-in `relative_ratio_baseline_floors`
  для `sub-ms` latency и `near-zero` lock-wait metrics, чтобы blocking verdict не
  зависел от measurement jitter при сохранении абсолютных ceilings и fail-closed budgets.
