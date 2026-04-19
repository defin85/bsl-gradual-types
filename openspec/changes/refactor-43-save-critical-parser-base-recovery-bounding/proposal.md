# Change: bound save-critical parser-base recovery for same-version exact readiness and preserve incident timeout-leaf fidelity

## Why

The fresh observability incident bundle captured at `2026-04-19T14:34:41.582Z` on
`0.4.155` / `git fa87144a` changes the diagnosis again.

The new bundle shows:

- completion itself is not the main bottleneck on this incident family:
  the hottest authoritative completion trace is `304ms`, while client pre-write and transport
  receive waits stay in the `1-23ms` range;
- both `didSave` heavy follow-up cycles publish only at `10246ms` / `10321ms`;
- both cycles remain `followup_ready_snapshot_task_state=in_flight_same_version`,
  exhaust bounded wait plus relief valve, and fall back through `followup_semantic_path=shadow_state`;
- raw diagnostics-save traces no longer stop at an opaque parse bucket:
  they identify `followup_ready_snapshot_timeout_leaf=parser_base_recovery`;
- cumulative metrics show `ready_parse_snapshot_materialization_ms origin=did_change`
  regressed to `p50/p95=4938/5103ms`, far above the accepted `p56` evidence refreshed earlier on
  `2026-04-19` (`234/267ms`);
- downstream `bsl.getCurrentContext` then spends most of its cost on
  `parse_source=parser_coordinator` with `parse_ms p50/p95=6194/10241` and
  `wall_ms p50/p95=9212/10342`;
- the authoritative raw trace contains `followup_ready_snapshot_timeout_leaf=parser_base_recovery`,
  but derived `incident.json` drops that field even though neighboring timeout checkpoint facts
  survive.

This means the recent attribution work did its job: the incident is no longer primarily "opaque
parse latency" and no longer a UI/transport-first problem. The next change should attack the
runtime bottleneck inside save-critical `parser_base_recovery` and repair the derived export drift
that hides the same leaf from request-centric handoff.

## What Changes

- Require same-version `didSave` follow-up to promote and keep the still-current same-version
  exact background producer on the save-critical `parser_base_recovery` path, so the producer
  does not spend representative steady-state latency trapped in parser-base recovery before
  `ready_artifacts` can materialize.
- Require refreshed representative evidence to show that the `conf_big` incident family no longer
  regresses into multi-second `didChange` materialization lag plus `shadow_state` fallback solely
  because `parser_base_recovery` monopolized the same-version exact path.
- Require downstream evidence that the healed path again makes same-version ready snapshots
  available early enough that `bsl.getCurrentContext` does not keep defaulting to
  `parser_coordinator` on the same representative family.
- Require observability incident bundle projection to preserve low-cardinality diagnostics-save
  timeout-leaf facts from the authoritative raw trace into derived `incident.json` and
  human-readable `summary.md`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
  - backend/runtime and extension incident-bundle regression coverage
- Follow-up relationship:
  - builds on `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
  - builds on `refactor-41-ready-snapshot-before-first-parse-exec-subphase-bounding`
  - is intentionally separate from
    `refactor-42-restore-lsp-exact-consumers-after-diagnostics-only-simplification`
  - does not reopen VS Code UI / extension pre-send latency unless fresh contradictory evidence
    appears

## Non-Goals

- Do not widen bounded wait or relief-valve budgets as the primary remedy.
- Do not redesign `bsl.getCurrentContext` broker semantics as the first-line fix if restoring
  same-version exact readiness removes the downstream parser-coordinator pressure.
- Do not ship a bundle-only cosmetics fix without addressing the runtime `parser_base_recovery`
  regression that produced this incident class.
