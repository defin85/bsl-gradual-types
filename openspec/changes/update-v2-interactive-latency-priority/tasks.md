## 1. Implementation
- [x] 1.1 Добавить в v2 orchestration разделение freshness-policy по классу операции: interactive (`completion/hover/signatureHelp`) vs diagnostics.
- [x] 1.2 Добавить runtime knobs для interactive freshness policy: `intellisense_v2_interactive_wait_budget_ms`, `intellisense_v2_interactive_max_stale_version_gap`, `intellisense_v2_interactive_max_stale_age_ms`.
- [x] 1.2.1 Добавить валидацию/clamp runtime knobs в допустимые диапазоны и метрику `intellisense_v2_interactive_knob_clamped_total`.
- [x] 1.3 Реализовать bounded wait + controlled stale fallback для интерактивных операций (включая fast-exit при отсутствии допустимого stale snapshot) без изменения внешних LSP payload.
- [x] 1.3.1 Явно зафиксировать и реализовать политику stale mismatch: snapshot с несовпадающими `deps_id/settings_id` не используется.
- [x] 1.4 Сохранить strict latest publish policy для diagnostics и гарантировать drop stale результата по `file_version/deps_id/settings_id`.
- [x] 1.5 Добавить singleflight-дедупликацию для revision-bound query (`parse_result`, `syntax_diagnostics`, `ir`) с очисткой in-flight записей и корректной обработкой follower-cancel.
- [x] 1.5.1 Зафиксировать политику ошибок singleflight: followers получают терминальный outcome leader, auto-retry внутри того же flight отсутствует.
- [x] 1.6 Добавить class-aware priority budgeting для blocking CPU-пути с отдельными permits для `interactive` и `background`.
- [x] 1.6.1 Добавить borrow-правило permits при пустой очереди противоположного класса с восстановлением fairness при конкуренции.
- [x] 1.7 Расширить observability метрики для stale/singleflight/queue-class и зафиксированных ключей контракта.

## 2. Validation
- [x] 2.1 Добавить/обновить интеграционные тесты LSP на сценарий “долгий diagnostics + интерактивный запрос”.
- [x] 2.2 Добавить/обновить тесты на fast-exit интерактивного пути, когда stale snapshot недоступен по лимитам.
- [x] 2.3 Добавить/обновить тесты на singleflight для одинаковой ревизии (shared-result и cleanup after completion/error/cancel).
- [x] 2.4 Добавить/обновить тесты на отсутствие stale diagnostics publish.
- [x] 2.5 Добавить/обновить тесты fairness-budget: interactive не starve при load diagnostics и diagnostics не голодает при потоке interactive.
- [x] 2.5.1 Добавить/обновить тесты borrow-поведения permits и возврата fairness при двусторонней нагрузке.
- [x] 2.6 Прогнать `cargo test -p bsl-backend --bin bsl-lsp-server`.
- [x] 2.7 Прогнать `cargo test --workspace`.
- [x] 2.8 Добавить/обновить perf smoke quality gate на warm-path SLO (`50` completion запросов, `p95` wait/completion).

## 3. Spec & Ops Check
- [x] 3.1 Проверить, что observability snapshot содержит все зафиксированные ключи stale/singleflight/priority.
- [x] 3.2 Обновить внутреннюю документацию по latency policy (LSP v2).
- [x] 3.3 `openspec validate update-v2-interactive-latency-priority --strict --no-interactive`.
