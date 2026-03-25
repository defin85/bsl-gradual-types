# Трассируемость

## Требование -> Код -> Тест

### Superseded active completion освобождает interactive ownership до завершения stale response-build

- Требование:
  `openspec/changes/refactor-completion-superseded-active-turn-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_active_turn_during_response_build`
  `backend/src/bin/lsp_server/server/core/tests.rs::p40_real_conf_big_same_file_overlap_completion_perf_report_live`

### Response-build имеет cooperative cancellation checkpoints внутри тяжёлого tail

- Требование:
  `openspec/changes/refactor-completion-superseded-active-turn-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `bsl-runtime/src/application/type_system/services/completion_service.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_active_turn_during_response_build`
  `backend/src/bin/lsp_server/server/core/tests.rs::p40_real_conf_big_same_file_overlap_completion_perf_report_live`

### Старый request сохраняет bounded stale outcome и не публикует поздний user-facing completion

- Требование:
  `openspec/changes/refactor-completion-superseded-active-turn-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p28_cancel_request_stops_completion_and_prevents_late_publish`
  `backend/src/bin/lsp_server/server/core/tests.rs::p28_newer_completion_proactively_cancels_older_active_completion_on_same_file`
  `backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_active_turn_during_response_build`

### Representative real-module gate включает overlap profile для same-file supersession

- Требование:
  `openspec/changes/refactor-completion-superseded-active-turn-release/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/core/tests.rs`
  `scripts/validate-completion-superseded-active-turn-release.sh`
  `scripts/validate-v2-completion-gates.sh`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p40_real_conf_big_same_file_overlap_completion_perf_report_live`

### Shipped verification path знает о новом overlap profile

- Требование:
  `openspec/changes/refactor-completion-superseded-active-turn-release/tasks.md`
  `openspec/changes/refactor-completion-superseded-active-turn-release/validation/overlap-gate.md`
- Код:
  `scripts/run-intellisense-tests.sh`
  `scripts/test-intellisense-readiness-assets.py`
  `scripts/validate-completion-superseded-active-turn-release.sh`
  `scripts/validate-v2-completion-gates.sh`
- Тест:
  `python3 -m unittest scripts/test-intellisense-smoke-gate.py scripts/test-intellisense-readiness-assets.py`
