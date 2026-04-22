## 1. Save-critical exact `program_lowering`

- [x] 1.1 Bound the exact same-version `program_lowering` path so the save-critical producer is no
      longer forced to behave like one monolithic lowering span before first exact follow-up
      publish.
- [x] 1.2 Preserve fail-closed exactness and truthful supersession / retarget behavior when a newer
      same-file target arrives while the producer is inside the new bounded lowering checkpoints.

## 2. Conversion attribution coherence

- [x] 2.1 Make diagnostics-save conversion attribution internally coherent for one traced target
      and cycle, so `program_conversion_ms` cannot be exported smaller than
      `program_lowering_ms` or `publishable_artifact_packaging_ms` in the same trace.
- [x] 2.2 Export the coherent conversion attribution through diagnostics save timeline / incident
      bundle surfaces without regressing the truthful phase-, subphase-, core-build-, and
      assembly-checkpoint attribution added by `refactor-23` through `refactor-30`.

## 3. Regressions and live evidence

- [x] 3.1 Add backend regressions for:
      bounded save-critical exact `program_lowering`,
      non-regression of supersession / retarget behavior inside the new lowering checkpoints,
      and coherent aggregate conversion attribution across repeated follow-up probe snapshots.
- [x] 3.2 Capture representative repo-local live evidence on `examples/conf_big` showing whether
      the mixed `didChange + didSave` path returns to `ready_artifacts`, or which exact
      `program_lowering` residual remains dominant after the fix, while also proving that the
      exported conversion aggregate stays internally coherent.

## 4. Validation

- [x] 4.1 Run targeted backend tests covering bounded exact `program_lowering`, coherent
      conversion attribution, and the relevant `didSave` follow-up path.
- [x] 4.2 Run VS Code diagnostics-save request / incident-bundle tests if the timeline contract or
      bundle rendering changes.
- [x] 4.3 Run `openspec validate refactor-31-diagnostics-save-exact-program-lowering-bounding --strict --no-interactive`.

## 5. OpenSpec / Beads Sync

- [x] 5.1 Keep the next Beads epic and its child tasks aligned with the implementation status and
      dependency graph of this change once execution begins.
