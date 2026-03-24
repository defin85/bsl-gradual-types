# Representative mixed-load gate

## Область
- Change: `refactor-document-symbol-interactive-isolation`
- Профиль: `p39_real_conf_big_document_symbol_mixed_load_gate_live`
- Фокус: same-file `didChange`/`didSave` + burst `textDocument/documentSymbol` + `textDocument/completion` на real module

## Поставляемые пути
- Workflow: `.github/workflows/ci.yml`
- Локальный script: `./scripts/validate-v2-completion-gates.sh`
- Aggregate report:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-readiness-gate.json`
- Aggregate summary:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-readiness-gate.md`
- Real-module report:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.json`
- Checked-in summary:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.md`
- OpenSpec validate log:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-openspec-validate.log`

## Проверка
- `CHANGE_ID=refactor-document-symbol-interactive-isolation ./scripts/validate-v2-completion-gates.sh`
- `cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`
- `openspec validate refactor-document-symbol-interactive-isolation --strict --no-interactive`

## Результат
- На 24 марта 2026 года checked-in mixed-load gate для `refactor-document-symbol-interactive-isolation` зелёный на `master`.
- Measured set:
  `10` completion samples, `10` `head_hit`, `0` `exact_hit`,
  `0` `prepare_timeout`, `0` `exact_deadline`, `0` ingress-regression samples.
- Outline companion path в том же прогоне дал:
  `40` `latest_ready`, `0` `current_ready`, `0` `unavailable`, `0` `superseded`,
  `40` non-null responses и `0` null responses.
- Interactive ingress budget сохранён:
  `p95(service_future_to_first_poll_wait_ms)=23ms`,
  `max(service_future_to_first_poll_wait_ms)=23ms`,
  `p95(transport_to_handler_wait_ms)=23ms`,
  `max(transport_to_handler_wait_ms)=23ms`,
  runtime budget `intellisense_v2_interactive_wait_budget_ms=120`.
- Это даёт acceptance evidence, что auxiliary outline refresh больше не превращает completion ingress в starvation point даже под same-file mixed load.

## Связь с требованиями
- Requirement: auxiliary `documentSymbol` traffic не starving interactive semantic admission
  Code: `backend/src/bin/lsp_server/server/language_server/impl_features_a.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  `backend/src/bin/lsp_server/server/core.rs`
  Test:
  `cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap -- --nocapture`
  `cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`
- Requirement: representative gate детерминированно ловит outline-induced starvation
  Code: `backend/src/bin/lsp_server/server/core/tests.rs`
  `.github/workflows/ci.yml`
  `scripts/validate-v2-completion-gates.sh`
  `scripts/README.md`
  Evidence:
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.json`
  `backend/tests/perf/reports/refactor-document-symbol-interactive-isolation-real-conf-big-document-symbol-mixed-load-live.md`
