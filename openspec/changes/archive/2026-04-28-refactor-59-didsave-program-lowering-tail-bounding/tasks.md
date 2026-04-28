## 1. Contract and Evidence

- [x] 1.1 Record the post-refactor-58 bundle evidence from
      `/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z`
      and keep it linked from the change.
- [x] 1.2 Compare the new bundle against the `refactor-08` live report and the
      `2026-04-26T21-01-14Z` pre-refactor-58 bundle, preserving the exact
      before/after metrics used for the scope decision.
- [x] 1.3 State explicit non-goals for `refactor-57`, `refactor-58`,
      completion/UI dispatch, current-context routing, and budget widening.

## 2. Instrumentation

- [x] 2.1 Audit diagnostics-save timeline fields for the post-refactor-58 v15
      path and ensure dominant program-lowering tails export reuse outcome,
      rebuilt/reused units, reuse-plan hit flags, or an explicit missing-evidence
      gap.
- [x] 2.2 Refine incident-bundle projection so a small measured
      `snapshot_with_deps_ms` plus seconds-scale `program_lowering` does not
      look like a generic `snapshot_with_deps` blocker.
- [x] 2.3 Add or update bundle gap/guard coverage for missing program-lowering
      reuse evidence when `timeout_leaf=program_lowering` dominates a same-file
      didSave follow-up.
- [x] 2.4 Extend VS Code custom request and incident-bundle raw/summary
      projection so program-lowering reuse outcome, rebuilt/reused unit counts,
      and reuse-plan hit flags survive from backend timeline evidence into the
      exported operator bundle.

## 3. Runtime Behavior

- [x] 3.1 Audit exact ready-snapshot assembly/program-lowering reuse on the
      same-version `didSave` save-critical path after refactor-58.
- [x] 3.2 Bound the representative `program_lowering` tail, or emit a truthful
      required-full-rebuild / supersession / cancellation / failure /
      continuity-loss reason instead of accepting a generic readiness bucket.
- [x] 3.3 Preserve fast `save_fastlane` first publish, completion isolation, and
      current-context attribution while changing the didSave tail.

## 4. Tests

- [x] 4.1 Add focused diagnostics-save timeline regression coverage for
      post-refactor-58 `detached_ready_artifacts` with a dominant
      `program_lowering` tail and non-dominant `ready_install`.
- [x] 4.2 Add VS Code custom request and incident-bundle projection coverage
      proving the residual is reported as exact assembly/program-lowering
      materialization tail, with reuse evidence preserved, not only generic
      `snapshot_with_deps`.
- [x] 4.3 Add negative/guard coverage proving missing program-lowering reuse
      evidence is a validation gap when program lowering dominates.

## 5. Validation

- [x] 5.1 Run the focused backend/LSP tests added or touched by this change.
- [x] 5.2 Run the relevant VS Code extension custom-request and projection tests
      if incident-bundle projection changes.
- [x] 5.3 Capture a fresh representative incident bundle or equivalent live
      report and verify:
      - observability contract violations remain absent or `0`;
      - invalid saturation metric violations remain absent or `0`;
      - completion ingress/egress remains bounded;
      - current-context timeline remains available;
      - didSave full follow-up no longer has an unclassified seconds-scale
        program-lowering tail hidden under generic `snapshot_with_deps`.
- [x] 5.4 Run `cargo check --workspace --all-targets`.
- [x] 5.5 Run `cargo clippy --workspace --all-targets -- -D warnings` if
      production Rust changes are made.
- [x] 5.6 Run
      `openspec validate refactor-59-didsave-program-lowering-tail-bounding --strict --no-interactive`.
