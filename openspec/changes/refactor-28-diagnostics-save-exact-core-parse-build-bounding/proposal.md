# Change: bound exact `core_parse_build` on the same-version `didSave` ready-snapshot path

## Why

`refactor-27` removed the opaque `parse_exec` blob and proved two things on the representative
`conf_big` mixed `didChange + didSave` path:

- save-critical promotion can already cut through deferrable optional cache-enrichment work;
- the remaining exact-path timeout is still truthful, but now it is narrower:
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
  - `followup_ready_snapshot_parse_exec_dominant_subphase=core_parse_build`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

At this point another change that only widens wait budgets or reopens optional enrichment would be
misdirected. The dominant residual is exact same-version `core_parse_build` itself.

## What Changes

- Require the exact same-version ready-snapshot producer to keep `core_parse_build` on a
  save-critical budget, so publishable exact ready artifacts do not wait for secondary build work
  that can be deferred past first publish.
- Require bounded attribution inside `core_parse_build`, so operator-facing evidence can distinguish
  which exact core-build slice still dominates when `conf_big` does not return to
  `ready_artifacts`.
- Require regressions and repo-local live evidence that show either a return to `ready_artifacts`
  on the mixed path, or a new truthful residual below the old `core_parse_build` bucket.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`
- `refactor-27-diagnostics-save-exact-parse-exec-bounding`

`refactor-25` removed parser-base drift as the primary root cause.
`refactor-26` removed apply-lag as the primary exact-path blocker.
`refactor-27` split `parse_exec` and proved that `core_parse_build` is now the dominant residual.
This change targets what remains after all three: exact same-version core build latency on the
save-critical path.

## Epic

This change is tracked by Beads epic `bsl-gradual-types-zb0m`
(`OpenSpec refactor-28: exact core_parse_build bounding`).

Execution children for this step:

- `bsl-gradual-types-zb0m.1` - save-critical exact core_parse_build
- `bsl-gradual-types-zb0m.2` - bounded core-build checkpoints and attribution
- `bsl-gradual-types-zb0m.3` - regressions and conf_big live evidence
- `bsl-gradual-types-zb0m.4` - targeted validation and strict OpenSpec validation

Dependency graph:

- `bsl-gradual-types-zb0m.1` starts first;
- `bsl-gradual-types-zb0m.2` depends on `.1`;
- `bsl-gradual-types-zb0m.3` depends on `.1` and `.2`;
- `bsl-gradual-types-zb0m.4` depends on `.3`.

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
- Do not reopen optional cache-enrichment work except where needed to keep attribution truthful.
- Do not relax exact same-version guarantees or publish stale diagnostics.
