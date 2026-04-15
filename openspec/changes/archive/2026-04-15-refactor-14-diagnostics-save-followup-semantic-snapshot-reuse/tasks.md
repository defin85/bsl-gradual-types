## 1. Implementation
- [x] 1.1 Add deterministic regressions proving that `didSave + idle_heavy` semantic follow-up
      still pays direct parse/IR cost even when same-version `SetFileWithSnapshot` state is
      available, and that an already-ready same-version parse snapshot is currently not preferred
      over `shadow_state`.
- [x] 1.2 Rework `analysis-v2` semantic diagnostics profiled helpers so they use snapshot-aware
      parse-result and IR accessors, preserve same-version correctness, and keep truthful profile
      accounting for parse/IR source.
- [x] 1.3 Rework didSave heavy follow-up branch selection so an already-ready same-version parse
      snapshot is preferred immediately over `shadow_state`, without adding a new long wait budget
      before fallback and without weakening supersession/latest-wins behavior.
- [x] 1.4 Extend request-centric didSave save timeline contract from `v7` to `v8` with bounded
      semantic path and parse/IR source attribution, and update incident-bundle / VS Code
      projections with explicit degradation semantics for older payloads.

## 2. Validation
- [x] 2.1 Run targeted regressions for snapshot-backed semantic diagnostics reuse, ready-artifacts
      branch preference, truthful fallback on stale or missing snapshots, and diagnostics save
      timeline `v8` contract fields.
- [x] 2.2 Capture representative `conf_big` evidence showing reduced
      `semantic_diagnostics_query_parse_result_ms` and explicit snapshot-backed source attribution
      in didSave follow-up.
- [x] 2.3 Run `openspec validate refactor-14-diagnostics-save-followup-semantic-snapshot-reuse --strict --no-interactive`.
