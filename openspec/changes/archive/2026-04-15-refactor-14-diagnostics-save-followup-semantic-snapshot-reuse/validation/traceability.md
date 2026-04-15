# Traceability

## Requirement -> Code -> Test

| Requirement | Code | Test / Evidence |
|---|---|---|
| same-version `didSave + idle_heavy` semantic follow-up MUST reuse snapshot-aware parse/IR accessors instead of forcing direct salsa parse/IR recompute on snapshot-backed analysis state. | `analysis-v2/src/lib/analysis_api.rs` | `analysis-v2/src/lib/tests.rs` (`semantic_diagnostics_profiled_report_snapshot_parse_and_ir_sources`) |
| `didSave` heavy follow-up MUST prefer already-ready same-version `ready_artifacts` before `shadow_state`, while preserving truthful fallback when reuse is unavailable. | `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` | `cargo test -p bsl-backend p7_did_save_fastlane_followup_publishes_full_diagnostics_from_ready_artifacts_before_delayed_apply -- --nocapture` `cargo test -p bsl-backend p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts -- --nocapture` |
| diagnostics save timeline MUST publish bounded semantic path / parse-source / IR-source attribution through the additive `v8` contract, and older consumers MUST degrade explicitly instead of inferring missing reuse. | `backend/src/bin/lsp_server/types.rs` `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts` | `backend/src/bin/lsp_server/server/core/tests.rs` (`p45_real_conf_big_did_save_diagnostics_followup_syntax_report_live`) `vscode-extension/src/test/suite/customRequests.test.ts` (`getDiagnosticsSaveTimeline should work via executeCommand`) |
| representative repo-local evidence MUST show same-version `didSave` follow-up publishing through `ready_artifacts` with snapshot-backed semantic input on `conf_big`. | `backend/tests/perf/reports/refactor-14-diagnostics-save-followup-semantic-snapshot-reuse-real-conf-big-did-save-diagnostics-followup-syntax-live.json` | `openspec/changes/refactor-14-diagnostics-save-followup-semantic-snapshot-reuse/validation/followup-live.md` |

## Acceptance Closure

- As of `2026-04-12`, the checked-in implementation matches `tasks.md` `1.1` through `2.3`.
- Snapshot-backed semantic diagnostics now route through `AnalysisV2` snapshot-aware parse / IR accessors instead of bypassing them with direct salsa calls on the default same-version path.
- `didSave` idle-heavy follow-up prefers already-ready same-version `ready_artifacts` immediately and falls back to truthful `shadow_state` only when same-version reuse is absent or stale.
- The diagnostics save timeline contract publishes explicit semantic attribution via `v8`, and the VS Code incident-bundle projection treats older payloads as unavailable-by-design rather than silently omitting reuse state.

## OpenSpec / Beads Sync

- `tasks.md` already reflects the implemented state: all checklist items are marked done.
- Strict validation passes:
  `openspec validate refactor-14-diagnostics-save-followup-semantic-snapshot-reuse --strict --no-interactive`
- `bsl-gradual-types-tr73.1` through `bsl-gradual-types-tr73.7` were stale-open despite the code and evidence being present; they are closed on `2026-04-12` to match OpenSpec reality.
- Epic `bsl-gradual-types-tr73` is closed on `2026-04-12` after child sync.
