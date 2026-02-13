## 1. Implementation
- [ ] 1.1 Добавить в v2 orchestration разделение freshness-policy по классу операции: interactive (`completion/hover/signatureHelp`) vs diagnostics.
- [ ] 1.2 Добавить runtime knobs для interactive freshness policy: `intellisense_v2_interactive_wait_budget_ms`, `intellisense_v2_interactive_max_stale_version_gap`, `intellisense_v2_interactive_max_stale_age_ms`.
- [ ] 1.3 Реализовать bounded wait + controlled stale fallback для интерактивных операций (включая fast-exit при отсутствии допустимого stale snapshot) без изменения внешних LSP payload.
- [ ] 1.4 Сохранить strict latest publish policy для diagnostics и гарантировать drop stale результата по `file_version/deps_id/settings_id`.
- [ ] 1.5 Добавить singleflight-дедупликацию для revision-bound query (`parse_result`, `syntax_diagnostics`, `ir`) с очисткой in-flight записей и корректной обработкой follower-cancel.
- [ ] 1.6 Добавить class-aware priority budgeting для blocking CPU-пути с отдельными permits для `interactive` и `background`.
- [ ] 1.7 Расширить observability метрики для stale/singleflight/queue-class и зафиксированных ключей контракта.

## 2. Validation
- [ ] 2.1 Добавить/обновить интеграционные тесты LSP на сценарий “долгий diagnostics + интерактивный запрос”.
- [ ] 2.2 Добавить/обновить тесты на fast-exit интерактивного пути, когда stale snapshot недоступен по лимитам.
- [ ] 2.3 Добавить/обновить тесты на singleflight для одинаковой ревизии (shared-result и cleanup after completion/error/cancel).
- [ ] 2.4 Добавить/обновить тесты на отсутствие stale diagnostics publish.
- [ ] 2.5 Добавить/обновить тесты fairness-budget: interactive не starve при load diagnostics и diagnostics не голодает при потоке interactive.
- [ ] 2.6 Прогнать `cargo test -p bsl-backend --bin bsl-lsp-server`.
- [ ] 2.7 Прогнать `cargo test --workspace`.

## 3. Spec & Ops Check
- [ ] 3.1 Проверить, что observability snapshot содержит все зафиксированные ключи stale/singleflight/priority.
- [ ] 3.2 Обновить внутреннюю документацию по latency policy (LSP v2).
- [ ] 3.3 `openspec validate update-v2-interactive-latency-priority --strict --no-interactive`.
