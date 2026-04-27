## 1. Contract and Evidence

- [x] 1.1 Record the post-refactor-59 bundle evidence from
      `/home/egor/code/temp/bsl-observability-incident-2026-04-27T11-07-23Z`
      and keep it linked from the change.
- [x] 1.2 Compare the new bundle against
      `/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z`,
      preserving the exact deltas used for the scope decision.
- [x] 1.3 State explicit non-goals for UI/pre-send, completion transport,
      runtime saturation, current-context routing, budget widening, and global
      unbounded AST retention.

## 2. Runtime Architecture

- [x] 2.1 Audit exact `didSave` lowering reuse seed selection in
      `bsl-runtime/src/system/parser_coordinator.rs`, including owned cache,
      borrowed cache, same-content plan, ranged-edit plan, and existing
      save-family/didChange parse-snapshot sources.
- [x] 2.2 Introduce a bounded save-family lowering reuse seed or equivalent
      continuity source that does not depend solely on opportunistic AST cache
      residency.
- [x] 2.3 Define deterministic seed selection order and validation rules for
      same-file didSave exact assembly.
- [x] 2.4 Keep full rebuild legal only when the runtime records a
      low-cardinality required-full-rebuild, supersession, cancellation,
      failure, or continuity-loss reason.
- [x] 2.5 Define bounded seed retention and cleanup rules for terminal,
      superseded, cancelled, failed, and capacity-pressure outcomes.
- [x] 2.6 Preserve fast `save_fastlane` first publish and existing exact
      interactive readiness semantics.

## 3. Observability

- [x] 3.1 Export seed source and/or reuse-plan failure reason in the backend
      diagnostics-save timeline when `program_lowering` dominates exact
      assembly.
- [x] 3.2 Export seed candidate count and eviction reason so bounded retention
      cannot masquerade as an accepted full-rebuild explanation.
- [x] 3.3 Preserve the new fields through VS Code custom request typing,
      incident-bundle raw JSON, and human-readable summary.
- [x] 3.4 Update bundle gaps so `full_rebuild` with
      `reuse_plan_build_source=null` is fail-visible unless a reason proves the
      rebuild was required.

## 4. Tests

- [x] 4.1 Add parser-coordinator coverage for save-family seed reuse when the
      opportunistic AST cache would otherwise miss.
- [x] 4.2 Add negative coverage for unsafe/missing seed cases that must produce
      explicit reasons instead of silent full rebuild.
- [x] 4.3 Add diagnostics-save timeline regression coverage for the v11/v15
      contrast: one save can reuse `2088/0`, and a later same-file save must not
      silently fall back to `0/2088` with no source/reason.
- [x] 4.4 Add seed retention cleanup coverage proving terminal/superseded/
      cancelled/failed save families release seeds without leaking memory.
- [x] 4.5 Add negative coverage proving normal steady-state bounded-retention
      eviction of the active save-family seed is not an accepted success for the
      representative large-module save profile.
- [x] 4.6 Add VS Code incident-bundle projection coverage for seed source,
      candidate count, eviction reason, and
      failure reason fields if projection changes.

## 5. Validation

- [x] 5.1 Run the focused backend/runtime tests added or touched by this change.
- [x] 5.2 Run the relevant VS Code extension custom-request and projection tests
      if incident-bundle projection changes.
- [x] 5.3 Capture a fresh representative incident bundle or equivalent live
      report and verify:
      - observability contract violations remain absent or `0`;
      - invalid saturation metric violations remain absent or `0`;
      - completion ingress/egress remains bounded;
      - v11-like successful lowering reuse remains fast;
      - no same-file didSave follow-up has an unproved seconds-scale
        `program_lowering_tail` with `full_rebuild`, `0` reused units, all units
        rebuilt, and no seed source/reason.
- [x] 5.4 Run `cargo fmt --check`.
- [x] 5.5 Run `cargo check --workspace --all-targets`.
- [x] 5.6 Run `cargo clippy --workspace --all-targets -- -D warnings` if
      production Rust changes are made.
- [x] 5.7 Run
      `openspec validate refactor-60-didsave-lowering-reuse-continuity --strict --no-interactive`.
