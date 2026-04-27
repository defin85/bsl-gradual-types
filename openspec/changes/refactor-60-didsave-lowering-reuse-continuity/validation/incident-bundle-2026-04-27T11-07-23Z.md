# Incident Bundle Evidence: 2026-04-27T11:07:23Z

Source bundle:
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T11-07-23Z`

## Runtime Identity

- Extension: `BSL Gradual Type System` `0.4.160`
- LSP server: `0.4.160 (build: 2026-04-27 13:54:10, git: 5691e618)`
- Binary path:
  `/home/egor/code/bsl-gradual-types/vscode-extension/bin/lsp-server`
- Captured at: `2026-04-27T11:07:23.244Z`

This is a valid post-refactor-59 runtime. It includes the committed p33
same-text current-context ready-snapshot follow-up fix, so the residual must not
be treated as an installed-runtime mismatch.

## Completion and Integrity

- Completion timeline: available, authoritative, 6 traces, contract `v25`.
- Completion duration histogram: `count=6`, `p50=0ms`, `p95=203ms`,
  `p99=203ms`.
- Slowest completion trace: `completion-trace-5`, `203ms`, dominant stage
  `collect`.
- Client pre-send remains small in local probes: write deltas `2-5ms`.
- Completion output handoff/write waits remain small: handoff send wait max
  `1ms`, write/flush max `1ms`.
- Completion fallback/stale counters:
  `intellisense_v2_completion_fallback_unavailable_total=0`,
  `intellisense_v2_completion_stale_fallback_total=0`.
- Observability integrity:
  `intellisense_v2_observability_contract_violation_total=0`; invalid
  saturation metric absent.
- Runtime saturation sample count: `498`; queue-depth gauge `0`.

The incident should therefore not reopen UI/pre-send, completion ingress,
completion output handoff, fallback, stale-read, or runtime saturation.

## didSave Timeline

Two same-file `didSave` traces were captured.

### Trace 1: Successful reuse shape

- Trace: `diagnostics-save-trace-1`
- Requested version: `11`
- First publish: `65ms`, `save_fastlane:syntax_only:published`
- Full follow-up: `2258ms`, `idle_heavy:full:published`
- Semantic path: `detached_ready_artifacts`
- Parse source: `snapshot`
- IR source: `snapshot_build`
- `snapshot_with_deps_ms=1153`
- `semantic_diagnostics_query_ms=1102`
- `parse_exec_ms=11`
- `exact_ready_snapshot_assembly_ms=1`
- `program_lowering_ms=1`
- `ready_install_ms=4`
- `reuse_outcome=top_level_reuse`
- `reused_lowering_units=2088`
- `rebuilt_lowering_units=0`
- `reuse_plan_build_source=borrowed`
- `reuse_plan_take_if_unique_hit=false`
- `reuse_plan_borrowed_cache_hit=true`
- `followup_readiness_blocker_bucket=snapshot_with_deps`

This trace proves the runtime can perform a fast, high-coverage lowering reuse
for this file family.

### Trace 2: Residual continuity failure

- Trace: `diagnostics-save-trace-2`
- Requested version: `15`
- First publish: `62ms`, `save_fastlane:syntax_only:published`
- Full follow-up: `4649ms`, `idle_heavy:full:published`
- Semantic path: `detached_ready_artifacts`
- Parse source: `snapshot`
- IR source: `snapshot_build`
- `snapshot_with_deps_ms=0`
- `semantic_diagnostics_query_ms=574`
- `followup_ready_snapshot_wait_probe=timeout`
- `timeout_phase=parse_exec`
- `timeout_leaf=program_lowering`
- `parse_exec_ms=4128`
- `exact_ready_snapshot_assembly_ms=4125`
- `program_conversion_ms=4125`
- `program_lowering_ms=4125`
- `ready_install_ms=1`
- `reuse_outcome=full_rebuild`
- `reused_lowering_units=0`
- `rebuilt_lowering_units=2088`
- `reuse_plan_build_source=null`
- `reuse_plan_take_if_unique_hit=false`
- `reuse_plan_borrowed_cache_hit=false`
- `relief_valve_outcome=engaged_timed_out`
- `relief_valve_budget_ms=500`
- `followup_readiness_blocker_bucket=program_lowering_tail`

This is the new target residual. Classification now points to
`program_lowering_tail`, but the runtime still has no save-cycle-local proof of
why the later same-file save could not reuse lowering from an earlier successful
family member.

## Current Context Timeline

- Current-context timeline: available, authoritative, 9 traces.
- Terminal counts: 4 resolved, 5 superseded.
- Parse sources:
  - `ready_snapshot`: 1 resolved trace at `127ms`, `parse_ms=0`;
  - `parser_coordinator`: 3 resolved traces at `4147ms`, `4783ms`, `5864ms`;
  - superseded traces: 5.
- Broker leader role appears in 8 traces.

Current-context coverage is still imperfect, but it is not the primary didSave
residual in this bundle. The didSave v15 tail is inside exact
`program_lowering`.

## Comparison Against 2026-04-27T08:39:19Z

The previous bundle ran git `033ac549`; the new bundle runs `5691e618`.

- Completion p95 improved from `282ms` to `203ms`.
- Completion local probe duration for the slow non-empty completion improved
  from `285ms` to `216ms`.
- The refactor-59 classifier effect is visible:
  - previous v15 blocker: `snapshot_with_deps`;
  - new v15 blocker: `program_lowering_tail`.
- The residual didSave shape remains:
  - previous v15 `program_lowering=3596ms`, `full_rebuild`, `0/2088`;
  - new v15 `program_lowering=4125ms`, `full_rebuild`, `0/2088`.
- The missing source remains the key clue:
  - previous v15 `reuse_plan_build_source=null`;
  - new v15 `reuse_plan_build_source=null`,
    `take_if_unique_hit=false`, `borrowed_cache_hit=false`.

## Scope Decision

The next change should focus on didSave lowering reuse continuity:

- make a save-family reuse seed explicit and bounded;
- stop relying solely on opportunistic AST cache residency for save-critical
  lowering reuse;
- require full rebuild to carry a reason;
- preserve completion and fast first-publish behavior.
