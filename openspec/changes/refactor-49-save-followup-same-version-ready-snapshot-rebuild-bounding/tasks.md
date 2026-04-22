## 1. Contract

- [ ] 1.1 Add a `bsl-intellisense-v2` requirement that same-version `didSave` heavy follow-up
      MUST avoid cold exact ready-snapshot rebuild as the default primary gate when the saved
      revision is still current and safe same-version reuse inputs already exist.
- [ ] 1.2 Define representative acceptance that fails if `examples/conf_big` still lands on
      `followup_semantic_path=shadow_state` because same-version rebuild remains dominated by
      `parse_exec/exact_ready_snapshot_assembly/program_lowering` rather than by newer revision or
      separately attributed queue/apply blockers.

## 2. Design

- [ ] 2.1 Describe the exact target identity and the safe same-version reuse inputs that may seed
      a faster exact ready-snapshot rebuild.
- [ ] 2.2 Describe truthful fallback behavior when reuse proof is absent, mismatched, or
      superseded by a newer save target.
- [ ] 2.3 Describe the representative live/perf evidence and worst-outlier correlation slice that
      proves rebuild-stage latency, not queue/apply lag, is removed.

## 3. Implementation

- [ ] 3.1 Introduce a same-version `didSave` exact ready-snapshot rebuild fast path or a
      semantically equivalent reuse-aware reduction that prevents seconds-scale cold rebuild from
      being the default path for still-current saved revisions.
- [ ] 3.2 Keep detached diagnostics-ready consumption, canonical live exact install, and
      interactive exact fail-closed semantics correct on top of the faster rebuild path.
- [ ] 3.3 Add regressions for same-version saved-revision rebuild, supersession/mismatch behavior,
      and truthful rebuild-stage timeout attribution.
- [ ] 3.4 Refresh representative live evidence on `examples/conf_big` showing that heavy follow-up
      no longer falls back to `shadow_state` solely because same-version rebuild stayed dominated
      by `program_lowering`-class exact assembly work.

## 4. Validation

- [ ] 4.1 Run targeted backend/runtime/diagnostics-save regressions for the new same-version
      rebuild fast path and preserved fail-closed semantics.
- [ ] 4.2 Run representative live/perf validation for the `didSave` same-version rebuild gate on
      `examples/conf_big`.
- [ ] 4.3 Run `openspec validate refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding --strict --no-interactive`.
