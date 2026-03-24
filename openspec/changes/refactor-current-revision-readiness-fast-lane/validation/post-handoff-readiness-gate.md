# Post-Handoff Readiness Gate

## Scope
- Change: `refactor-current-revision-readiness-fast-lane`
- Profile: `p38_real_conf_big_post_handoff_readiness_completion_perf_report_live`
- Focus: current-revision apply/head readiness after same-file `didChange` handoff

## Shipped Paths
- Workflow: `.github/workflows/ci.yml`
- Local script: `./scripts/validate-v2-completion-gates.sh`
- Real-module report path:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.json`

## Verification
- `openspec validate refactor-current-revision-readiness-fast-lane --strict --no-interactive`
- `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`

## Result
- On March 24, 2026 both commands passed on `master`.
- The gate now records the readiness evidence under the readiness-fast-lane change id instead of the split-prepare follow-up change id.
- This keeps producer-side post-handoff readiness evidence distinct from later split-prepare shipped gate closure.

## Traceability
- Requirement: operation-aware current-revision readiness polling
  Code: `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`, `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  Test: `cargo test -p bsl-runtime interactive_snapshot_for_completion_preempts_background_backlog -- --nocapture`
- Requirement: completion current-revision snapshot keeps deps/index truth atomic
  Code: `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  Test: `cargo test -p bsl-runtime completion_current_revision_snapshot -- --nocapture`
- Requirement: real-module post-handoff readiness gate remains green with readiness-fast-lane change id
  Code: `backend/src/bin/lsp_server/server/core/tests.rs`, `.github/workflows/ci.yml`, `scripts/validate-v2-completion-gates.sh`, `scripts/README.md`
  Test: `CHANGE_ID=refactor-current-revision-readiness-fast-lane BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT=/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.json cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`
