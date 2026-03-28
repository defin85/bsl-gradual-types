# Representative mixed-load gate

## Область
- Change: `isolate-completion-pre-dispatch-ingress`
- Профиль: `p39_real_conf_big_document_symbol_mixed_load_gate_live`
- Фокус: same-file `didChange`/`didSave` + burst `textDocument/documentSymbol` + `textDocument/completion` на real module с явным pre-dispatch split `adapter_read_at_ms -> jsonrpc_dispatch_received_at_ms`

## Поставляемые пути
- Default smoke:
  `./scripts/run-intellisense-tests.sh smoke`
- Workflow: `.github/workflows/ci.yml`
- Локальный script:
  `./scripts/validate-isolate-completion-pre-dispatch-ingress.sh`
- Generic override path:
  `CHANGE_ID=isolate-completion-pre-dispatch-ingress ./scripts/validate-v2-completion-gates.sh`
- Aggregate report:
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-readiness-gate.json`
- Aggregate summary:
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-readiness-gate.md`
- Real-module report:
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-real-conf-big-document-symbol-mixed-load-live.json`
- Checked-in summary:
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-real-conf-big-document-symbol-mixed-load-live.md`
- OpenSpec validate log:
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-openspec-validate.log`

## Проверка
- `./scripts/validate-isolate-completion-pre-dispatch-ingress.sh`
- `CHANGE_ID=isolate-completion-pre-dispatch-ingress ./scripts/validate-v2-completion-gates.sh`
- `./scripts/run-intellisense-tests.sh smoke`
- `cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`
- `openspec validate isolate-completion-pre-dispatch-ingress --strict --no-interactive`

## Результат
- На 28 марта 2026 года checked-in mixed-load gate для `isolate-completion-pre-dispatch-ingress` зелёный в текущем workspace.
- Measured set:
  `10` completion samples, `10` `head_hit`, `0` `exact_hit`,
  `0` `prepare_timeout`, `0` `exact_deadline`,
  `0` pre-dispatch samples over budget и `0` pre-dispatch samples over hard cap.
- Outline companion path в том же прогоне дал:
  `40` `latest_ready`, `0` `current_ready`, `0` `unavailable`, `0` `superseded`,
  `40` non-null responses и `0` null responses.
- Pre-dispatch ingress budget сохранён:
  `p95(adapter_to_dispatch_wait_ms)=1ms`,
  `max(adapter_to_dispatch_wait_ms)=1ms`,
  `p95(service_future_to_first_poll_wait_ms)=3ms`,
  `max(service_future_to_first_poll_wait_ms)=3ms`,
  `p95(transport_to_handler_wait_ms)=3ms`,
  `max(transport_to_handler_wait_ms)=3ms`,
  runtime budget `intellisense_v2_interactive_wait_budget_ms=120`.
- Отчёт теперь отделяет server-side pre-dispatch backlog от post-dispatch first-poll/handler wait и больше не описывает этот класс задержки как client-side ingress.
- Trace summaries прямо показывают `adapter_read_at_ms`, `adapter_to_dispatch_wait_ms`, `jsonrpc_dispatch_received_at_ms` и legacy `transport_received_at_ms`, поэтому расследование не требует ручного вычитания timestamp'ов.

## Связь с требованиями
- Requirement: representative gate fail-closed ловит возвращение completion starvation именно в окне `adapter read -> dispatch`
  Code: `backend/src/bin/lsp_server/server/core/tests.rs`
  Test:
  `cargo test -p bsl-backend --bin bsl-lsp-server p39_real_conf_big_document_symbol_mixed_load_gate_live -- --nocapture`
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-real-conf-big-document-symbol-mixed-load-live.json`
- Requirement: change-specific readiness wrapper публикует truthful pre-dispatch split и aggregate evidence bundle
  Code: `scripts/validate-isolate-completion-pre-dispatch-ingress.sh`
  `scripts/validate-v2-completion-gates.sh`
  `scripts/README.md`
  `docs/agent/verification.md`
  Test:
  `./scripts/validate-isolate-completion-pre-dispatch-ingress.sh`
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-readiness-gate.json`
  `backend/tests/perf/reports/isolate-completion-pre-dispatch-ingress-readiness-gate.md`
