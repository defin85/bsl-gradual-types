# Final Readiness Evidence (dual-artifact completion path)

## Scope
- change_id: `refactor-v2-completion-dual-artifact-path`
- date (UTC): `2026-03-18T07:23:48+00:00`

## Requirement -> Code -> Test

### 1. IntelliSense v2 обеспечивает `head-or-exact-or-fail-closed` completion без stale substitute
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `analysis-v2/src/derived_artifacts.rs`
- Test evidence:
  - `cargo test -p bsl-backend --bin bsl-lsp-server p33_ -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p37_real_conf_big_warm_cache_completion_perf_report_live -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` -> `ok`
  - Fresh artifacts:
    - `backend/tests/perf/reports/real-conf-big-warm-cache-completion-perf-live.json`
    - `backend/tests/perf/reports/real-conf-big-revision-churn-completion-perf-live.json`

### 2. Completion head и exact path используют один canonical current-revision source of truth
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `analysis-v2/src/derived_artifacts.rs`
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
- Test evidence:
  - `cargo test -p bsl-runtime completion_head_receiver_ -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server completion_head_observation_invalidation -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p33_ -- --nocapture` -> `ok`

### 3. Completion bounded wait/freshness остаётся current-revision и fail-closed при отсутствии head/exact artifacts
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
- Test evidence:
  - `cargo test -p bsl-backend --bin bsl-lsp-server p33_ -- --nocapture` -> `ok`
  - Artifact: `backend/tests/perf/reports/completion-deadline-recovery-perf.json`
  - Fresh report summary:
    - `first_outcome=fail_closed`
    - `deadline_total=1`
    - `second_outcome=ok_non_empty`
    - `ready_total=1`

### 4. Non-completion interactive semantics остаются exact-or-fail-closed
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`
  - `bsl-agent/src/session/manager_semantic_core.rs`
- Test evidence:
  - `cargo test -p bsl-backend --bin bsl-lsp-server p7_hover_cache_miss_emits_bounded_fail_closed_reason -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p7_hover_and_definition_do_not_backfill_from_runtime_index_snapshot -- --nocapture` -> `ok`
  - `cargo test -p bsl-agent semantic_helpers_fail_closed_without_precomputed_type_index -- --nocapture` -> `ok`

### 5. Latency budget защищается canonical fast path, а не fallback semantics
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `bsl-runtime/src/system/basic_observability/tests.rs`
- Test evidence:
  - `cargo test -p bsl-runtime completion_route_fail_closed_cause_and_upgrade_metrics_are_recorded -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p37_real_conf_big_warm_cache_completion_perf_report_live -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` -> `ok`
  - Fresh live summaries:
    - warm `p95=8ms`, `head_hit_total=9`, `fail_closed_total=0`
    - revision-churn `p95=19ms`, `measured_head_hit_traces=4`, `measured_fail_closed_traces=0`

### 6. Representative real-module gate является acceptance source of truth
- Requirement: `openspec/changes/refactor-v2-completion-dual-artifact-path/specs/bsl-intellisense-v2/spec.md`
- Code:
  - `backend/src/bin/lsp_server/server/core/tests.rs`
- Test evidence:
  - `cargo test -p bsl-backend --bin bsl-lsp-server p37_real_conf_big_warm_cache_completion_perf_report_live -- --nocapture` -> `ok`
  - `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` -> `ok`
  - Artifacts:
    - `backend/tests/perf/reports/real-conf-big-warm-cache-completion-perf-live.json`
    - `backend/tests/perf/reports/real-conf-big-revision-churn-completion-perf-live.json`

## Contract/Test Evidence
- `cargo test -p bsl-runtime completion_head_receiver_ -- --nocapture` -> `ok`
- `cargo test -p bsl-runtime completion_route_fail_closed_cause_and_upgrade_metrics_are_recorded -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server completion_head_observation_invalidation -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server p33_ -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_hover_cache_miss_emits_bounded_fail_closed_reason -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_hover_and_definition_do_not_backfill_from_runtime_index_snapshot -- --nocapture` -> `ok`
- `cargo test -p bsl-agent semantic_helpers_fail_closed_without_precomputed_type_index -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server p37_real_conf_big_warm_cache_completion_perf_report_live -- --nocapture` -> `ok`
- `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture` -> `ok`

## Acceptance Artifacts
- Command logs:
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-runtime-completion-head-receiver.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-runtime-completion-route-metrics.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-completion-head-observation-invalidation.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-p33-suite.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-p7-hover-fail-closed.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-p7-hover-definition-no-search-rescue.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-agent-semantic-helpers-fail-closed.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-p37-real-conf-big-warm.log`
  - `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/cargo-test-bsl-backend-p38-real-conf-big-revision-churn.log`
- Generated reports:
  - `backend/tests/perf/reports/completion-deadline-recovery-perf.json`
  - `backend/tests/perf/reports/real-conf-big-warm-cache-completion-perf-live.json`
  - `backend/tests/perf/reports/real-conf-big-revision-churn-completion-perf-live.json`

## OpenSpec Validate Evidence
- Command: `openspec validate refactor-v2-completion-dual-artifact-path --strict --no-interactive`
- Result: `Change 'refactor-v2-completion-dual-artifact-path' is valid`
- Raw log: `openspec/changes/refactor-v2-completion-dual-artifact-path/validation/refactor-v2-completion-dual-artifact-path-openspec-validate.log`
