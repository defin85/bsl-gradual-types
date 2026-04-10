# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| Interactive completion readiness waits must register as passive waiters without seconds-scale residency in unrelated apply backlog before the request becomes observable. | `bsl-runtime/src/application/intellisense_v2/facade.rs` `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs` `backend/src/bin/lsp_server/server/core/tests.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p33_completion_waiter_registration_bypasses_unrelated_interactive_apply_backlog -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-revision-churn-completion-perf-live.json` |
| didSave heavy follow-up must use the same passive readiness contract and must not inherit raw generic runtime FIFO residency before it can wait for the requested revision or equivalent ready state. | `bsl-runtime/src/application/intellisense_v2/facade.rs` `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs` `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` `backend/src/bin/lsp_server/server/core/tests.rs` | `cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_followup_stays_isolated_from_generic_background_reserved_blocker -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live -- --nocapture` `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-did-save-diagnostics-followup-runtime-live.json` |
| Observability must keep waiter registration / passive wait distinct from actual apply lag and downstream semantic work so completion and didSave bundles remain diagnosable. | `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs` `bsl-runtime/src/application/intellisense_v2/facade/tests.rs` `backend/src/bin/lsp_server/server/core/tests.rs` | `cargo test -p bsl-runtime wait_for_file_version_registration_bypasses_background_backlog_before_passive_wait -- --nocapture` `validation/observability-evidence.md` `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-revision-churn-completion-perf-live.json` `backend/tests/perf/reports/refactor-12-runtime-ready-waiter-contention-bounding-real-conf-big-did-save-diagnostics-followup-runtime-live.json` |

## OpenSpec / Beads sync

- `tasks.md` now matches delivered work for `1.1` through `2.5`.
- Strict validation passes:
  `openspec validate refactor-12-runtime-ready-waiter-contention-bounding --strict --no-interactive`
- Change-scoped Beads graph remains the execution mirror for this rollout:
  `bsl-gradual-types-mnp8` `bsl-gradual-types-mnp8.1` `bsl-gradual-types-mnp8.2` `bsl-gradual-types-mnp8.3` `bsl-gradual-types-mnp8.4` `bsl-gradual-types-mnp8.5` `bsl-gradual-types-mnp8.6` `bsl-gradual-types-mnp8.7` `bsl-gradual-types-mnp8.8` `bsl-gradual-types-mnp8.9`
