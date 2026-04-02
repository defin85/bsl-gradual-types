# Трассируемость

## Требование -> Код -> Тест

### Event-driven completion освобождает transport slot до длительного passive `turn_wait`

- Требование:
  `openspec/changes/refactor-completion-turn-wait-slot-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/main.rs`
  `backend/src/bin/lsp_server/server/mod.rs`
  `backend/src/bin/lsp_server/server/transport_adapter.rs`
  `backend/src/bin/lsp_server/server/request_context.rs`
- Тест:
  `backend/src/bin/lsp_server/server/transport_adapter.rs::transport_adapter_releases_ingress_slot_before_blocking_completion_wait`
  `backend/src/bin/lsp_server/server/core/tests.rs::p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live`
  `./scripts/validate-completion-turn-wait-slot-release.sh`

### Post-handoff completion сохраняет single-owner и exactly-once terminal semantics

- Требование:
  `openspec/changes/refactor-completion-turn-wait-slot-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/transport_adapter.rs`
  `backend/src/bin/lsp_server/server/core.rs`
  `backend/src/bin/lsp_server/server/completion_cancellation.rs`
  `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
- Тест:
  `backend/src/bin/lsp_server/server/transport_adapter.rs::transport_adapter_emits_single_terminal_response_for_handoff_cancel_race`
  `backend/src/bin/lsp_server/server/transport_adapter.rs::transport_adapter_aborts_blocked_completion_handoff_on_transport_shutdown`
  `backend/src/bin/lsp_server/server/core/tests.rs::p28_cancel_request_releases_pre_active_turn_wait_before_active_registration`
  `backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration`

### Completion timeline отделяет off-transport wait от ingress backlog

- Требование:
  `openspec/changes/refactor-completion-turn-wait-slot-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/transport_adapter.rs`
  `backend/src/bin/lsp_server/server/request_context.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  `backend/src/bin/lsp_server/types.rs`
  `vscode-extension/src/lsp/customRequests.ts`
  `vscode-extension/src/providers/completionTimelineModel.ts`
  `vscode-extension/src/providers/observabilityIncidentBundle.ts`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p22_get_completion_timeline_contains_completion_trace`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::server_edge_details_derive_first_poll_and_first_wake_split_when_present`
  `./scripts/run-intellisense-tests.sh smoke`
  `python3 -m unittest scripts/test-intellisense-smoke-gate.py scripts/test-intellisense-readiness-assets.py`

### Same-file overlap gate ловит completion `turn_wait` transport-slot retention

- Требование:
  `openspec/changes/refactor-completion-turn-wait-slot-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/core/tests.rs`
  `scripts/validate-completion-turn-wait-slot-release.sh`
  `scripts/validate-v2-completion-gates.sh`
  `.github/workflows/ci.yml`
  `docs/agent/verification.md`
  `docs/guides/development-workflow.md`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live`
  `python3 -m unittest scripts/test-intellisense-readiness-assets.py`
  `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-readiness-gate.json`
  `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-readiness-gate.md`
  `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-real-conf-big-pre-active-overlap-completion-perf-live.json`
  `backend/tests/perf/reports/refactor-completion-turn-wait-slot-release-real-conf-big-pre-active-overlap-completion-perf-live.md`
