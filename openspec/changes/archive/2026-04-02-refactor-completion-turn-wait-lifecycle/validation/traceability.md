# Трассируемость

## Требование -> Код -> Тест

### Superseded completion в `turn_wait` не становится orphaned до active registration

- Требование:
  `openspec/changes/refactor-completion-turn-wait-lifecycle/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  `backend/src/bin/lsp_server/server/core.rs`
- Тест:
  `backend/src/bin/lsp_server/server/completion_dispatcher/tests.rs::newer_request_supersedes_pre_active_turn_wait_request_before_active_registration`
  `backend/src/bin/lsp_server/server/completion_dispatcher/tests.rs::explicit_cancel_stops_pre_active_turn_wait_request_before_active_registration`
  `backend/src/bin/lsp_server/server/core/tests.rs::p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration`
  `backend/src/bin/lsp_server/server/core/tests.rs::p28_cancel_request_releases_pre_active_turn_wait_before_active_registration`

### Completion timeline truthfully отражает `turn_wait` lifecycle текущего request и stale contenders

- Требование:
  `openspec/changes/refactor-completion-turn-wait-lifecycle/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  `scripts/run-intellisense-tests.sh`
  `scripts/test-intellisense-readiness-assets.py`
- Тест:
  `backend/src/bin/lsp_server/server/completion_dispatcher/tests.rs::turn_waiter_preserves_non_zero_absolute_lifecycle_after_observed_wait`
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs::turn_attribution_trace_preserves_turn_wait_resolution_timestamps`
  `./scripts/run-intellisense-tests.sh smoke`
  `python3 -m unittest scripts/test-intellisense-smoke-gate.py scripts/test-intellisense-readiness-assets.py`

### Same-file overlap gate ловит stranded pre-active `turn_wait` request

- Требование:
  `openspec/changes/refactor-completion-turn-wait-lifecycle/specs/bsl-intellisense-v2/spec.md`
- Код:
  `backend/src/bin/lsp_server/server/core/tests.rs`
  `scripts/validate-completion-turn-wait-lifecycle.sh`
  `scripts/validate-v2-completion-gates.sh`
  `.github/workflows/ci.yml`
  `scripts/README.md`
  `docs/agent/verification.md`
- Тест:
  `backend/src/bin/lsp_server/server/core/tests.rs::p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live`
  `./scripts/validate-completion-turn-wait-lifecycle.sh`
  `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-readiness-gate.json`
  `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-readiness-gate.md`
  `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-real-conf-big-pre-active-overlap-completion-perf-live.json`
  `backend/tests/perf/reports/refactor-completion-turn-wait-lifecycle-real-conf-big-pre-active-overlap-completion-perf-live.md`
