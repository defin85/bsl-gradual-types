# Change: decouple `didSave` diagnostics follow-up from live exact install via detached ready artifacts

## Why

The latest `2026-04-20` representative evidence narrows the remaining `didSave` save-followup
regression again.

After the parser-base recovery work, the representative same-version path no longer spends its
bounded window inside `parser_base_recovery`. The still-current cycle now reaches parse/lowering
work quickly, but the heavy follow-up still times out in `ready_install` and falls back through
`shadow_state`.

The current evidence shows:

- `followup_ready_snapshot_parse_exec_ms=241ms`;
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms=224ms`;
- `followup_ready_snapshot_dominant_phase=ready_install`;
- `followup_ready_snapshot_ready_install_ms=4005ms`;
- `followup_ready_snapshot_timeout_leaf=ready_install`;
- `followup_semantic_path=shadow_state`;
- current-revision exact/head readiness is still absent at timeout
  (`current_type_index_parse_snapshot_meta_after_timeout=None`,
  `completion_head_ready_after_timeout=false`,
  `exact_ready_after_timeout=false`).

That means the representative `didSave` incident no longer needs a generic parse fix first.
It needs a safer way for diagnostics follow-up to consume the already-built same-version
diagnostics-ready payload without waiting for canonical live exact install.

One attempted shortcut already failed: publishing snapshot-backed state into live readiness before
the exact install wait regressed initial `didOpen` ready publication and was reverted. So the fix
must not weaken live exact gates for interactive consumers just to unblock diagnostics follow-up.

## What Changes

- Require a detached diagnostics-ready artifact for same-version `didSave` follow-up, keyed by the
  current save target identity, so the bounded follow-up can consume already-built diagnostics
  payload before live exact install completes.
- Keep this detached artifact diagnostics-only: `hover`, `definition`, `signatureHelp`,
  `type-at-position`, completion exact upgrade, and other interactive exact consumers MUST remain
  on the existing canonical live exact-readiness gate.
- Require truthful supersession, cancellation, and fallback semantics when a newer same-file
  revision or newer save cycle overtakes the detached artifact target.
- Require operator-facing observability to distinguish detached diagnostics-ready consumption from
  canonical live `ready_artifacts` and from degraded `shadow_state` fallback.
- Add backend regressions and refreshed representative evidence for the `ready_install`-dominated
  same-version save-followup family, with `p55` / `p56`-style save-followup traces as the
  representative acceptance surface.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `analysis-v2` artifact/read-model boundary or semantically equivalent detached storage surface
  - representative live evidence and diagnostics-followup regressions
- Follow-up relationship:
  - builds on the `2026-04-20` root-cause update in
    `refactor-43-save-critical-parser-base-recovery-bounding`
  - assumes `refactor-43-save-critical-parser-base-recovery-bounding` remains the owner of
    save-critical `parser_base_recovery` boundedness and derived diagnostics-save timeout-leaf
    fidelity
  - is intentionally narrower than `refactor-current-revision-head-detached-snapshot`; this change
    is only about `didSave` diagnostics follow-up, not a general detached completion/read model

## Non-Goals

- Do not satisfy the change by early-publishing partial snapshot-backed state into canonical live
  exact readiness.
- Do not widen bounded `didSave` wait or relief-valve budgets as the primary remedy.
- Do not let detached diagnostics-ready artifacts become silent substitutes for interactive exact
  consumers.
- Do not redesign the broader current-revision detached head/completion architecture in this
  change.
- Do not reopen parser-base dominance or raw-to-derived timeout-leaf fidelity work already scoped
  by `refactor-43-save-critical-parser-base-recovery-bounding`.
- Do not treat the legacy `p53_real_conf_big_exact_program_lowering_report_live` probe as a
  required acceptance gate for this change; `p53` may remain as an optional diagnostic signal, but
  the operational representative gates for this scope are the late save-followup surfaces.
