# Change: bound exact `program_conversion` inside same-version `ready_snapshot_assembly`

## Why

`refactor-29` removed deferred syntax-error assembly from the save-critical exact path and narrowed
the live `conf_big` residual below the broader `exact_ready_snapshot_assembly` bucket.

The checked-in `refactor-29` live evidence now shows a tighter, truthful exact-path bottleneck:

- `followup_semantic_path=shadow_state`
- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=program_conversion`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms≈4034`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=null`

At this point widening wait budgets again would be misdirected. The dominant residual is now exact
same-version `program_conversion` itself.

## What Changes

- Require the exact same-version ready-snapshot producer to keep `program_conversion` on the
  save-critical path, so first publishable exact ready artifacts do not wait for secondary
  conversion or packaging work that can happen after first publish.
- Require bounded attribution inside `program_conversion`, so operator-facing evidence can
  distinguish which exact conversion slice still dominates when representative mixed load does not
  return to `ready_artifacts`.
- Require regressions and repo-local live evidence that show either a return to `ready_artifacts`
  on the mixed `conf_big` path, or a new truthful residual below the old `program_conversion`
  bucket.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`
- `refactor-27-diagnostics-save-exact-parse-exec-bounding`
- `refactor-28-diagnostics-save-exact-core-parse-build-bounding`
- `refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding`

`refactor-25` removed parser-base drift as the primary root cause.
`refactor-26` removed apply-lag as the primary exact-path blocker.
`refactor-27` isolated `parse_exec`.
`refactor-28` isolated `tree_cache_install`.
`refactor-29` isolated deferred syntax-error assembly and proved that the remaining exact-path
timeout is now dominated by `program_conversion`.

This change targets what remains after all five: exact same-version `program_conversion` latency on
the save-critical path.

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
- Do not reopen `parser_tree_build`, `tree_cache_install`, or deferred `syntax_error_collection`
  as the primary focus.
- Do not relax exact same-version guarantees or publish stale diagnostics.
