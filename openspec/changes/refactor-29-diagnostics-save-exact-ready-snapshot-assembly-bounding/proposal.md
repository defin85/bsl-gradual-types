# Change: bound exact `ready_snapshot_assembly` on the same-version `didSave` ready-snapshot path

## Why

`refactor-28` removed `tree_cache_install` from the save-critical exact path and narrowed the live
`conf_big` residual below monolithic `core_parse_build`.

The checked-in `refactor-28` live evidence now shows a tighter, truthful exact-path bottleneck:

- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms≈4050`
- `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms≈54`
- `followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms=null`
- `followup_semantic_path=shadow_state`

At this point widening wait budgets again would be misdirected. The dominant residual is now exact
same-version `ready_snapshot_assembly` itself.

## What Changes

- Require the exact same-version ready-snapshot producer to keep `ready_snapshot_assembly` on the
  save-critical path, so first publishable exact ready artifacts do not wait for secondary
  assembly work that can happen after first publish.
- Require bounded attribution inside `ready_snapshot_assembly`, so operator-facing evidence can
  distinguish which exact assembly slice still dominates when representative mixed load does not
  return to `ready_artifacts`.
- Require regressions and repo-local live evidence that show either a return to `ready_artifacts`
  on the mixed `conf_big` path, or a new truthful residual below the old
  `exact_ready_snapshot_assembly` bucket.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`
- `refactor-27-diagnostics-save-exact-parse-exec-bounding`
- `refactor-28-diagnostics-save-exact-core-parse-build-bounding`

`refactor-25` removed parser-base drift as the primary root cause.
`refactor-26` removed apply-lag as the primary exact-path blocker.
`refactor-27` isolated `parse_exec`.
`refactor-28` isolated `tree_cache_install` and proved that the remaining exact-path timeout is now
dominated by `ready_snapshot_assembly`.

This change targets what remains after all four: exact same-version ready-snapshot assembly latency
on the save-critical path.

## Epic

This change is tracked by Beads epic `bsl-gradual-types-z7od`
(`OpenSpec refactor-29: exact ready_snapshot_assembly bounding`).

Execution children for this step:

- `bsl-gradual-types-z7od.1` - save-critical exact ready_snapshot_assembly
- `bsl-gradual-types-z7od.2` - bounded assembly checkpoints and attribution
- `bsl-gradual-types-z7od.3` - regressions and conf_big live evidence
- `bsl-gradual-types-z7od.4` - targeted validation and strict OpenSpec validation

Dependency graph:

- `bsl-gradual-types-z7od.1` starts first;
- `bsl-gradual-types-z7od.2` depends on `.1`;
- `bsl-gradual-types-z7od.3` depends on `.1` and `.2`;
- `bsl-gradual-types-z7od.4` depends on `.3`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - ready-snapshot phase attribution / incident bundle summary / `conf_big` live evidence

## Non-Goals

- Do not widen the base `didSave` bounded wait or relief-valve budget as the primary fix.
- Do not reopen parser-tree construction or tree-cache install as the primary focus.
- Do not relax exact same-version guarantees or publish stale diagnostics.
