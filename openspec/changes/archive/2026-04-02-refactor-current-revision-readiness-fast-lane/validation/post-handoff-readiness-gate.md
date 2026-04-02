# Post-Handoff Readiness Gate

## Scope
- Change: `refactor-current-revision-readiness-fast-lane`
- Profile: `p38_real_conf_big_post_handoff_readiness_completion_perf_report_live`
- Focus: current-revision apply/head readiness after same-file `didChange` handoff

## Shipped Paths
- Workflow: `.github/workflows/ci.yml`
- Local script: `./scripts/validate-v2-completion-gates.sh`
- Aggregate report path:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-readiness-gate.json`
- Aggregate summary:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-readiness-gate.md`
- Real-module report path:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.json`
- Checked-in summary:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-real-conf-big-revision-churn-completion-perf-live.md`
- OpenSpec validation log:
  `backend/tests/perf/reports/refactor-current-revision-readiness-fast-lane-openspec-validate.log`

## Verification
- `./scripts/validate-v2-completion-gates.sh`
- `openspec validate refactor-current-revision-readiness-fast-lane --strict --no-interactive`
- `cargo test -p bsl-backend --bin bsl-lsp-server p38_real_conf_big_revision_churn_completion_perf_report_live -- --nocapture`

## Result
- On March 24, 2026 the checked-in live report for `refactor-current-revision-readiness-fast-lane`
  stayed green on `master`.
- The report keeps producer-side readiness evidence under the readiness-fast-lane change id,
  separate from the later split-prepare closure.
- Measured set:
  `10` samples, `10` `head_hit` traces, `0` `prepare_timeout`,
  `0` `exact_deadline`, `0ms` max `wait_for_file_version_runtime.queue_wait_ms`.
- All measured samples used the fast-ready bypass path
  (`measured_wait_for_file_version_runtime_queue_wait_bypassed_fast_ready_samples=10`),
  so the gate no longer shows post-handoff apply backlog or post-apply head-gap regressions.

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
- Full matrix: `validation/traceability.md`
