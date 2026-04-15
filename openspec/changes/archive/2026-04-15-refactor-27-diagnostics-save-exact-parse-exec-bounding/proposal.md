# Change: bound exact `parse_exec` on the same-version `didSave` ready-snapshot path

## Why

`refactor-25` removed the stale parser-base root cause, and `refactor-26` isolated post-ready
`apply_lag` from the exact follow-up path. The representative `conf_big` incident still does not
return to `ready_artifacts`, but the residual is now narrow and truthful:

- `followup_ready_snapshot_zero_probe=not_ready`
- `followup_ready_snapshot_wait_probe=timeout`
- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

At this point the runtime is no longer blocked primarily by parser-base mismatch or writer/apply
gating. The remaining bottleneck is exact same-version `parse_exec` itself. Another change that
only widens wait budgets would just stretch latency without removing the root cause.

## What Changes

- Require the same-version ready-snapshot producer to treat exact `didSave` follow-up as a
  save-critical path inside `parse_exec`, deferring or cancelling non-essential work that is not
  required to materialize exact ready artifacts.
- Require bounded subphase checkpoints and attribution inside exact `parse_exec`, so timeouts are
  no longer an opaque single `parse_exec` blob and obsolete work can still stop at meaningful
  in-parse boundaries.
- Require regressions and repo-local live evidence that show whether `conf_big` returns to
  `ready_artifacts`, or, if not, which exact in-parse subphase remains dominant after the fix.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`

`refactor-25` removed the stale parser-base cause and bounded obsolete parse waste.
`refactor-26` removed apply-lag as the primary late blocker on the exact path.
This change targets what remains after both: exact same-version `parse_exec` duration on the
save-critical path.

## Epic

This change is tracked by Beads epic `bsl-gradual-types-d3t6`
(`OpenSpec refactor-27: exact parse_exec bounding`).

Execution children for this step:

- `bsl-gradual-types-d3t6.1` - save-critical exact `parse_exec`
- `bsl-gradual-types-d3t6.2` - bounded in-parse checkpoints and attribution
- `bsl-gradual-types-d3t6.3` - regressions and `conf_big` live evidence
- `bsl-gradual-types-d3t6.4` - targeted validation and strict OpenSpec validation

Dependency graph:

- `bsl-gradual-types-d3t6.1` starts first;
- `bsl-gradual-types-d3t6.2` depends on `.1`;
- `bsl-gradual-types-d3t6.3` depends on `.1` and `.2`;
- `bsl-gradual-types-d3t6.4` depends on `.3`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - exact ready-snapshot lifecycle / observability metrics / `conf_big` live evidence

## Non-Goals

- Do not widen the base `didSave` bounded wait or the temporary relief valve as the primary fix.
- Do not revisit `apply_lag` / publish gating except where needed to consume the new exact
  `parse_exec` outcome truthfully.
- Do not relax exact same-version guarantees or permit stale diagnostics to publish.
