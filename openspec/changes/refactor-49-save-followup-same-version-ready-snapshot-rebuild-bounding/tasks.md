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
- [ ] 2.3 Describe the representative live/perf evidence and worst-outlier correlation slice that
      proves rebuild-stage latency, not queue/apply lag, is removed. The remaining plan context is:
      the synthetic same-content ownership seam is fixed and the newer cycle-1
      `followup_wait_reason=pending_publish` stall is no longer the only observed verdict, but the
      representative `examples/conf_big` contour is still unstable on cycle 2: one rerun reaches a
      cold `parse_exec -> exact_ready_snapshot_assembly -> program_lowering` outlier, while the
      latest rerun still stalls earlier at `pending_publish` before any bounded semantic-path
      decision.

## 3. Implementation

- [x] 3.1 Introduce a same-version `didSave` exact ready-snapshot rebuild fast path or a
      semantically equivalent reuse-aware reduction that prevents seconds-scale cold rebuild from
      being the default path for still-current saved revisions.
- [x] 3.2 Keep detached diagnostics-ready consumption, canonical live exact install, and
      interactive exact fail-closed semantics correct on top of the faster rebuild path.
- [x] 3.3 Add regressions for same-version saved-revision rebuild, supersession/mismatch behavior,
      and truthful rebuild-stage timeout attribution.
- [ ] 3.4 Refresh representative live evidence on `examples/conf_big` showing that heavy follow-up
      no longer falls back to `shadow_state` solely because same-version rebuild stayed dominated
      by `program_lowering`-class exact assembly work, and no longer oscillates between the
      cycle-2 `program_lowering` outlier and the cycle-2 `pending_publish` publication-proof stall
      after the archived-trace fastlane-progress and snapshot-summary observability fixes.

## 4. Validation

- [x] 4.1 Run targeted backend/runtime/diagnostics-save regressions for the new same-version
      rebuild fast path and preserved fail-closed semantics.
- [ ] 4.2 Run representative live/perf validation for the `didSave` same-version rebuild gate on
      `examples/conf_big`, preserving the current known split:
      truthful `waiting` attribution is already fixed, the same-content ownership seam is fixed,
      archived-trace fastlane-progress proof now survives active-cycle archival, but the open
      acceptance blocker still oscillates on cycle 2 between an earlier `pending_publish` stall and
      a later representative exact rebuild outlier at
      `parse_exec -> exact_ready_snapshot_assembly -> program_lowering`.
- [x] 4.3 Run `openspec validate refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding --strict --no-interactive`.

## Current Working Note

- Completed in this pass: diagnostics-save fastlane progress now falls back from `active_cycles`
  to matching archived traces keyed by `(uri, requested_version, diagnostics_generation,
  save_cycle_sequence)`, the new archived-trace regression is green, and the representative `p56`
  rerun no longer stops first on cycle-1 `followup_wait_reason=pending_publish`. In the same pass,
  exact-worker phase snapshots now retain `program_lowering_summary` for in-flight probes, and the
  new snapshot-export regression is green.
- Next implementation step: localize why cycle 2 is still unstable across representative live
  reruns. On the same tree, one `p56` rerun reaches
  `followup_ready_snapshot_parse_exec_ms~=45.1s` with
  `exact_ready_snapshot_assembly/program_lowering_ms~=45.1s`, while the latest rerun fails
  earlier with `followup_wait_reason=pending_publish` before any bounded semantic-path decision.
- Current working hypothesis: the earlier cycle-1 publication-proof seam is fixed, but cycle-2
  save-follow-up still races between publication-proof visibility and the slow exact-worker path on
  the same save target, so the representative contour alternates between a live `pending_publish`
  stall and a cold `program_lowering` outlier.
