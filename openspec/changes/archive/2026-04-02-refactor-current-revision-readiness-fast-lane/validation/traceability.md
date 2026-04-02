# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| Current-revision readiness fast lane продвигает `applied_version` и `CompletionHeadArtifact` раньше slow enrich path | `bsl-runtime/src/application/intellisense_v2/facade/operations.rs` `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs` `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs` | `cargo test -p bsl-runtime interactive_snapshot_for_completion_preempts_background_backlog -- --nocapture` `cargo test -p bsl-runtime wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths -- --nocapture` |
| Completion under large-module churn использует bounded wait и fail-closed current-revision path без post-handoff apply/head regressions | `backend/src/bin/lsp_server/server/language_server/impl_completion.rs` `bsl-runtime/src/application/intellisense_v2/facade/operations.rs` | `cargo test -p bsl-runtime completion_current_revision_snapshot -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p33_current_revision_head_precompute_stays_available_under_background_cpu_saturation -- --nocapture` `cargo test -p bsl-backend --bin bsl-lsp-server p33_completion_head_hit_emits_exact_upgrade_when_background_exact_finishes -- --nocapture` |
| Representative real-module gate проверяет post-handoff readiness и не пропускает `prepare_timeout@wait_for_file_version` или post-apply `head_ready=false` `exact_deadline` | `backend/src/bin/lsp_server/server/core/tests.rs` `.github/workflows/ci.yml` `scripts/validate-v2-completion-gates.sh` `scripts/README.md` | `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.json` `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.md` `validation/post-handoff-readiness-gate.md` |

## OpenSpec / Beads sync
- `tasks.md` отражает фактическое состояние change: `3.2`, `3.3`, `3.4`, `3.5` закрыты.
- `beads` должен отражать ту же реальность:
  `bsl-gradual-types-gy9c.12` и `bsl-gradual-types-gy9c.15` можно закрывать после обновления этого change.
- Архивация change не является prerequisite для этой traceability sync и может быть выполнена позже отдельным шагом.
