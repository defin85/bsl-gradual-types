## 1. Contract

- [x] 1.1 Add a `bsl-intellisense-v2` requirement that same-version `didSave` heavy follow-up
      MUST avoid cold exact ready-snapshot rebuild as the default primary gate when the saved
      revision is still current and safe same-version reuse inputs already exist.
- [x] 1.2 Define representative acceptance that fails if `examples/conf_big` still lands on
      `followup_semantic_path=shadow_state` because same-version rebuild remains dominated by
      `parse_exec/exact_ready_snapshot_assembly/program_lowering` rather than by newer revision or
      separately attributed queue/apply blockers.

## 2. Design

- [x] 2.1 Describe the exact target identity and the safe same-version reuse inputs that may seed
      a faster exact ready-snapshot rebuild.
- [x] 2.2 Describe truthful fallback behavior when reuse proof is absent, mismatched, or
      superseded by a newer save target.
- [x] 2.3 Describe the representative live/perf evidence and worst-outlier correlation slice that
      proves rebuild-stage latency, not queue/apply lag, is removed. The checked-in validation now
      captures the accepted split: the representative detached `examples/conf_big` cycle stays
      query-dominated (`semantic_diagnostics_query_ms=1118` vs `parse_exec_ms=150`,
      `program_lowering_ms=137`) with no rebuild-dominated `parse_exec/program_lowering`
      `shadow_state` residual, while the remaining `shadow_state` cycles are separately attributed
      to the `waiting` bucket instead of exact rebuild work.

## 3. Implementation

- [x] 3.1 Introduce a same-version `didSave` exact ready-snapshot rebuild fast path or a
      semantically equivalent reuse-aware reduction that prevents seconds-scale cold rebuild from
      being the default path for still-current saved revisions.
- [x] 3.2 Keep detached diagnostics-ready consumption, canonical live exact install, and
      interactive exact fail-closed semantics correct on top of the faster rebuild path.
- [x] 3.3 Add regressions for same-version saved-revision rebuild, supersession/mismatch behavior,
      and truthful rebuild-stage timeout attribution.
- [x] 3.4 Refresh representative live evidence on `examples/conf_big` showing that heavy follow-up
      no longer falls back to `shadow_state` because same-version rebuild stayed dominated by
      `program_lowering`-class exact assembly work. The accepted representative contour may still
      include truthful `waiting`-phase `shadow_state` cycles, but detached cycles now prove
      query-dominated follow-up and no longer oscillate into a rebuild-dominated
      `parse_exec/program_lowering` residual.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime/diagnostics-save regressions for the new same-version
      rebuild fast path and preserved fail-closed semantics.
- [x] 4.2 Run representative live/perf validation for the `didSave` same-version rebuild gate on
      `examples/conf_big`, proving the final accepted split: waiting-phase `shadow_state` remains
      separately attributed and non-rebuild, detached cycles stay under the baseline
      `parse_exec/publish` ceilings, and no cycle reports a rebuild-dominated
      `parse_exec/program_lowering` `shadow_state` residual.
- [x] 4.3 Run `openspec validate refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding --strict --no-interactive`.

## Completion Note

- Final implementation closure adds two durable seams on top of the earlier observability fixes:
  `didChange` and `didSave` now discard stale completed previous-version type-index tasks before
  follow-up observation, and same-version `DidSaveFollowup` rebuilds with non-empty `parser_edits`
  may prime the parser AST cache from a safe previous ready snapshot instead of re-entering the
  old cold rebuild contour.
- The representative `p56` rerun captured on `2026-04-23` now passes with the accepted live
  split: cycle 2 reaches `followup_semantic_path=detached_ready_artifacts`,
  `followup_ready_snapshot_parse_exec_ms=150`,
  `followup_ready_snapshot_program_lowering_ms=137`, and
  `followup_publish_semantic_diagnostics_query_ms=1118`, while
  `rebuild_dominated_shadow_state_count=0` and the detached cycle exports
  `program_lowering_reuse_outcome=routine_body_reuse` with owned-plan reuse.
- Remaining `shadow_state` cycles in the same representative report are no longer rebuild
  residuals. They stay explicitly attributed to `followup_ready_snapshot_timeout_phase=waiting`
  and still eventually materialize the saved exact ready snapshot, so `refactor-49` now closes on
  the rebuild seam it set out to fix.
