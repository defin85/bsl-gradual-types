## Context

After `refactor-07`, diagnostics save timeline is now truthful about `didSave` follow-up:

- `save_fastlane` first publish is bounded;
- remaining `idle_heavy` stalls are no longer hidden behind false `apply_lag`.

However, live bundle `2026-04-07T23:20:28Z` shows a new dominant tail:

- first publish `save_fastlane:syntax_only:published@55ms`;
- follow-up publish `idle_heavy:full:published@4497ms`;
- `wait_for_file_version_ms=1531`;
- `syntax_diagnostics_query_ms=2886`;
- `semantic_diagnostics_query_ms=0`.

This means the current `idle_heavy` path still spends multi-second time redoing syntax work on
large files even when same-version syntax artifacts were already materialized earlier in the save
pipeline.

## Goals

- Reduce `didSave + idle_heavy` latency on large files by avoiding redundant same-version syntax
  work.
- Preserve same-version correctness, supersession semantics, and truthful diagnostics save timeline.
- Keep the optimization narrow to the `didSave` follow-up path.

## Non-Goals

- Do not change `save_fastlane` semantics.
- Do not weaken `idle_heavy` final diagnostics richness.
- Do not introduce non-request-centric aggregate-only observability.

## Decisions

### 1. didSave follow-up should reuse same-version syntax artifacts when already available

If the save cycle already has same-version syntax diagnostics / parse-backed syntax artifacts, the
`idle_heavy` follow-up should not rerun full-file syntax query as its first expensive step.

Preferred order:

1. reuse same-version syntax diagnostics/artifacts from ready/applied state;
2. run semantic diagnostics on top of that state;
3. only rerun syntax query when reuse is impossible or stale.

### 2. Timeline must explain whether syntax work was reused or recomputed

`diagnostics_save_timeline` should remain operator-useful after the optimization.

Follow-up publish facts should distinguish:

- syntax reused from same-version artifacts;
- syntax recomputed;
- residual waits (`apply_lag`, `semantic_work`, `pending_publish`, `superseded`).

### 3. Keep correctness fail-closed

If the server cannot prove that syntax artifacts match the requested save version, it must fall back
to the current truthful recompute path rather than publish stale diagnostics.

## Risks / Trade-offs

- Reusing syntax artifacts too aggressively could hide fresh syntax regressions if version matching
  is wrong.
- A narrow optimization may improve `conf_big` materially while leaving smaller workloads
  unchanged; that is acceptable.
- Additional trace fields may require another diagnostics save timeline contract bump.

## Validation

1. Regression: `didSave + idle_heavy` avoids redundant syntax recompute when same-version syntax
   artifacts are already available.
2. Regression: when reuse is impossible, the follow-up still falls back truthfully and preserves
   same-version correctness.
3. Live report: representative `conf_big` save flow shows materially reduced
   `syntax_diagnostics_query_ms` in `idle_heavy`, or explicit proof that syntax reuse was applied.
