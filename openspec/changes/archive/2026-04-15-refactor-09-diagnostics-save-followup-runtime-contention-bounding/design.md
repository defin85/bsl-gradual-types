## Context

After `refactor-08`, the diagnostics save pipeline behaves materially better:

- `save_fastlane` first publish is bounded;
- `idle_heavy` follow-up reuses same-version syntax artifacts instead of rerunning expensive full
  syntax work.

However, live bundle `2026-04-08T01:15:44Z` shows a new dominant tail on the same `conf_big`
workflow:

- `save_fastlane:syntax_only:published@56ms`;
- `idle_heavy:full:published@43224ms`;
- follow-up trace attribution only shows:
  - `wait_for_file_version_ms=11580`
  - `semantic_diagnostics_query_ms=169`
- cumulative metrics show:
  - `runtime_queue_wait_interactive_ms p95=17042`
  - `apply_change_set_file_exec_ms p95=17613`.

This means the main remaining delay is now runtime/apply contention after first publish, plus a
request-centric observability gap for the unattributed tail.

## Goals

- Prevent `didSave + idle_heavy` follow-up from spending seconds-scale latency behind shared
  interactive/runtime contention once same-version first freshness has already been delivered.
- Keep diagnostics save timeline truthful about where post-fastlane time is actually spent.
- Make incident bundles operator-useful without reconstructing request-level causes from p95/p99
  metrics.

## Non-Goals

- Do not change `save_fastlane` semantics or first-publish freshness guarantees.
- Do not weaken final `idle_heavy` diagnostics richness.
- Do not broaden this change to completion latency or unrelated auxiliary request classes.

## Decisions

### 1. Post-fastlane idle_heavy should not inherit shared interactive contention by default

Once a save cycle has already published a bounded same-version first refresh, its `idle_heavy`
follow-up should no longer treat shared interactive/runtime contention as the normal primary gate.

The implementation may use a dedicated execution class, a non-interactive path, or another bounded
follow-up path, but the observable contract must be:

- unrelated shared interactive backlog is not the default seconds-scale blocker anymore;
- if contention still happens, the request-centric trace says so explicitly.

### 2. Follow-up traces must expose runtime/apply contention separately

`diagnostics_save_timeline` should no longer leave a large terminal or in-flight tail as:

- `elapsed_ms` with no internal breakdown; or
- a generic `pending` / `semantic_work` label when the server already knows a more precise blocker.

The follow-up trace should separately expose, when known:

- runtime queue wait before heavy follow-up work;
- `apply_change(SetFile)` / writer-apply execution contention;
- `wait_for_file_version`;
- semantic query work;
- publish wait.

If the server still cannot fully explain part of the tail, the remaining gap should stay explicit
instead of being silently hidden behind a misleading blocker label.

### 3. Incident bundle summary should surface request-centric blocker breakdown

Bundle summary is useful only if it mirrors the authoritative diagnostics save trace.

When the backend provides follow-up contention breakdown, `summary.md` and `incident.json` should
show those request-level facts directly and should not force the operator to infer the root cause
from cumulative metrics snapshots.

## Risks / Trade-offs

- Moving heavy follow-up off the shared interactive path must not allow stale older-version
  diagnostics to publish after a newer save cycle.
- Adding more request-centric timing fields likely requires another diagnostics save timeline
  contract bump and tighter client/server version alignment.
- If the runtime still has a large unattributed tail after this change, the repo may need a deeper
  writer/apply instrumentation pass rather than another scheduling tweak.

## Validation

1. Regression: unrelated shared interactive/runtime backlog no longer dominates post-fastlane
   `idle_heavy` follow-up as the default primary gate.
2. Regression: terminal and in-flight diagnostics save traces expose follow-up runtime contention
   breakdown without collapsing seconds-scale latency into an unexplained tail when authoritative
   seams are available.
3. Live report: representative `conf_big` save flow shows bounded first publish and a truthful
   follow-up runtime breakdown, checked in as
   `p46_real_conf_big_did_save_diagnostics_followup_runtime_report_live`.
