# Validation

Date: 2026-04-06

## Mandatory checks

- `openspec validate refactor-completion-front-edge-exact-deadline-removal --strict --no-interactive`
  - Result: pass
- `CHANGE_ID=refactor-completion-front-edge-exact-deadline-removal ./scripts/validate-v2-completion-gates.sh`
  - Result: pass
  - Notes: the canonical full-bundle entry point now auto-selects representative perf profiles `large churn` and the change-specific real-module `front_edge` gate.
- `CHANGE_ID=refactor-completion-front-edge-exact-deadline-removal cargo test -p bsl-backend --bin bsl-lsp-server p42_real_conf_big_front_edge_completion_perf_report_live -- --nocapture`
  - Result: pass
  - Report: `backend/tests/perf/reports/refactor-completion-front-edge-exact-deadline-removal-real-conf-big-front-edge-completion-perf-live.json`

## Front-edge evidence

From `backend/tests/perf/reports/refactor-completion-front-edge-exact-deadline-removal-real-conf-big-front-edge-completion-perf-live.json`:

- `measured_successful_traces=10`
- `measured_fail_closed_traces=0`
- `measured_prepare_timeout_total_delta=0`
- `measured_exact_deadline_total_delta=0`
- `measured_cold_query_bundle_pool_wait_samples=0`
- `prepare_kind=shadow_current_revision_fast_path`
- measured outcomes are bounded `ok_empty`

This is the durable operator-facing evidence for the change:

- immediate same-file front-edge completion no longer regresses into hidden `exact_deadline`
- the profile now produces successful current-revision responses instead of all-fail-closed samples
- there is no residual cold `query_bundle_pool_wait` bucket in this representative live profile

## Full-bundle note

The generic readiness script now pins this change to `PERF_PROFILES="large churn"` and
`REAL_MODULE_PROFILES="front_edge"` when no explicit override is provided. That keeps the
full-bundle validation surface aligned with the change contract: immediate post-edit/save
front-edge readiness plus the large/churn representative matrix, without treating the unrelated
`small` application-layer matrix as mandatory acceptance for this remediation.
