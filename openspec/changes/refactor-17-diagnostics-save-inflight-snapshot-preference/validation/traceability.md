# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| didSave heavy follow-up MUST try bounded exact same-version ready-artifact wait before `shadow_state` only when an exact same-version task is currently in flight. | `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` | `cargo test -p bsl-backend p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state -- --nocapture` |
| didSave heavy follow-up MUST keep immediate truthful fallback when no exact same-version task exists for the requested revision. | `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` | `cargo test -p bsl-backend p7_did_save_followup_skips_bounded_wait_when_only_did_save_refresh_task_exists -- --nocapture` |
| Fail-closed behavior MUST remain bounded and freshness-aware for stale, superseded, cancelled, mismatched, or shadow-missing task states. | `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` | `cargo test -p bsl-backend p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts -- --nocapture` `cargo test -p bsl-backend p7_ready_parse_snapshot_probe_wait_decision_classifies_ -- --nocapture` |
| Representative evidence MUST show that `shadow_state+salsa` follow-up cycles are reduced only for the exact-task-in-flight case. | `backend/tests/perf/reports/refactor-17-diagnostics-save-inflight-snapshot-preference-did-save-followup-inflight-exact.json` | `openspec/changes/refactor-17-diagnostics-save-inflight-snapshot-preference/validation/followup-live.md` |

## Acceptance Closure

- As of 2026-04-12, `didSave` follow-up keeps the existing zero-budget ready probe, but the bounded wait now runs only for already-known exact same-version task evidence; a same-version refresh task seeded by the current `didSave` itself does not qualify.
- When the exact task is absent, the runtime skips speculative bounded waiting and falls back directly to truthful `shadow_state` or generic behavior.
- Freshness guards remain dominant: the bounded wait still terminates with explicit `timeout`, `cancelled`, `superseded`, `generation_mismatch`, or `version_mismatch` outcomes instead of publishing stale diagnostics.

## OpenSpec / Beads Sync

- `tasks.md` now matches the implemented and validated state for `1.1` through `3.1`.
- Strict validation passes:
  `openspec validate refactor-17-diagnostics-save-inflight-snapshot-preference --strict --no-interactive`
- `bd close bsl-gradual-types-1rkq.3 ...` completed on `2026-04-12`; the child now reports `status=closed`.
- `bd close bsl-gradual-types-1rkq ...` completed on `2026-04-12`; the umbrella epic now reports `status=closed`.
