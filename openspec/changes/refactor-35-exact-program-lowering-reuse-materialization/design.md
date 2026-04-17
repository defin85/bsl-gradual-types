## Context

The representative `p55` save-follow-up trace on `2026-04-17` now splits into two clearer residuals:

1. parse-side exact assembly still spends about `946ms` inside
   `exact_ready_snapshot_assembly -> program_lowering`;
2. semantic diagnostics still spend about `802ms` inside semantic IR work.

This change targets the parse-side residual first.

The available evidence already shows reuse detection is working:

- `program_lowering_reuse_outcome=top_level_reuse`
- `reused_lowering_units=2042`
- `fully_reused_top_level_node_count=314`
- `rebuilt_lowering_units=46`

The current code path still does too much work after that decision:

- `bsl-runtime/src/system/parser_coordinator.rs` derives the reuse plan and recursively rebases
  unchanged reused statements;
- `syntax/src/tree_sitter_adapter/mod.rs` stores reused `Statement` values and body windows in the
  reuse plan;
- `syntax/src/tree_sitter_adapter/statement_converter/mod.rs` and
  `syntax/src/tree_sitter_adapter/statement_converter/declarations.rs` clone reused statements back
  into the final `Program`.

That means representative local edits already qualify for reuse, but the final exact assembly still
pays clone-heavy materialization proportional to reused subtree size.

## Goals / Non-Goals

- Goals:
  - reduce exact `program_lowering` cost on representative same-file save follow-ups by removing
    clone-heavy reuse materialization from unchanged regions;
  - preserve exact same-version semantics, invalidation boundaries, latest-wins supersession, and
    truthful cancellation/retarget behavior;
  - keep representative reuse-versus-rebuild evidence truthful after ownership-based consumption.
- Non-Goals:
  - invent more aggressive reuse heuristics than `refactor-33` already proved safe;
  - redesign the entire AST ownership model in one step;
  - tune wait policy, follow-up lanes, or diagnostics semantic materialization in this change.

## Decisions

### 1. Optimize reuse materialization, not reuse detection

The latest evidence shows reuse already qualifies on representative same-file edits.
The remaining parser-side residual comes from how reused units are materialized, not from the
planner failing to find unchanged regions.

This change therefore targets "cheaper materialization of already-proven reuse", not "more reuse
cases".

### 2. Consume the reuse plan by ownership

The exact path should consume reused top-level statements and reusable callable-body prefix/suffix
windows by ownership.
That removes the second deep-clone step that currently copies unchanged `Statement` trees back into
the final `Program`.

Observability counters such as reused/rebuilt unit counts should be computed before the plan is
consumed, so the runtime does not need a second pass over cloned trees merely to describe what
happened.

### 3. Preserve the fail-closed invalidation boundaries from `refactor-33`

This change is not allowed to widen reuse eligibility.
If a region already requires rebuild under `refactor-33`, it still requires rebuild here.
Only the cost of materializing already-approved reused regions changes.

### 4. Preserve save-critical exact-path behavior

Ownership-based reuse must remain compatible with:

- save-critical promotion on same-version follow-ups;
- latest-wins supersession by newer same-file revisions or newer save cycles;
- truthful cancellation and retarget checks while exact assembly is in flight.

The change is not ready if lower clone cost comes at the price of stale exact artifacts or weaker
supersession guarantees.

### 5. Treat structural sharing as a follow-up, not as part of this change

If ownership-based consumption still leaves representative `program_lowering` above an acceptable
residual, a later change may introduce broader structural sharing or a shared lowered-node arena.
That is intentionally out of scope here.

The first step should stay narrow, defensible, and measurable against the current baseline.

## Alternatives Considered

### 1. Extend reuse detection further

Rejected.
The current representative trace already proves reuse detection is firing.
More aggressive eligibility would add soundness risk without addressing the clone-heavy
materialization cost already observed.

### 2. Introduce full structural sharing immediately

Rejected for now.
It changes ownership and lifetime surfaces much more broadly.
Ownership-based consumption is a smaller first step and should be validated before a wider redesign
is considered.

### 3. Add more checkpoints or wait-policy changes

Rejected.
The heavy follow-up already publishes through `ready_artifacts`.
More orchestration work would mostly relabel the same parse-side CPU hotspot.

## Validation Strategy

- Add targeted parser/runtime regressions proving exact semantic parity for reused top-level units
  and reused callable-body windows after ownership-based materialization.
- Preserve fail-closed regressions for ambiguous invalidation boundaries.
- Refresh representative `p53` / `p55` evidence and compare exact `program_lowering_ms` to the
  `2026-04-17` baseline.

## Quality Gates

- Representative `p55` still publishes through `ready_artifacts`.
- Representative evidence still exports truthful reuse-versus-rebuild counters for exact
  `program_lowering`.
- Representative exact `program_lowering_ms` is materially lower than the `2026-04-17` baseline.
- If the parser-side residual is not materially reduced, the change is not ready even if synthetic
  regressions pass.
