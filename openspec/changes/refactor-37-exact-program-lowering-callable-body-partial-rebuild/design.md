## Context

The parser-side representative residual changed after `refactor-35`.

By `2026-04-17`, live exact-path attribution already showed:

- top-level reuse is firing on the representative same-file save-follow-up path;
- ownership-based reuse materialization made reuse-plan build and reused-region work negligible;
- the remaining parser CPU is no longer in reuse materialization.

The decisive follow-up trace from `2026-04-17` now shows:

- `program_lowering_ms=1407`
- `rebuild_dispatch_callable_ms=1399`
- `rebuild_dispatch_callable_body_dispatch_ms=1399`
- `rebuild_dispatch_callable_body_dispatch_call_count=45`
- `rebuild_dispatch_callable_non_body_dispatch_ms=0`

That means the dominant parser-side residual is the recursive rebuild of one changed callable body.
The next defensible parser-side step is therefore to shrink rebuilt work inside that callable body,
not to spend more time on already-cheap reuse-plan materialization.

## Goals / Non-Goals

- Goals:
  - reduce exact `program_lowering` cost on representative same-file save follow-ups by rebuilding
    only the invalidated region inside one changed callable body when soundness can be proven;
  - preserve exact same-version semantics, fail-closed invalidation, latest-wins supersession,
    save-critical promotion, and truthful cancellation/retarget behavior;
  - export direct rebuilt callable-body observability so representative evidence explains why parser
    work dropped.
- Non-Goals:
  - widen top-level reuse eligibility beyond `refactor-33`;
  - redesign diagnostics semantic work covered by `refactor-36`;
  - introduce broad structural sharing or a lowered-node arena across the whole AST;
  - weaken body-local soundness boundaries just to avoid whole-callable rebuild.

## Decisions

### 1. Optimize rebuilt callable bodies, not reused regions

The current representative trace already proves reused regions are cheap enough after
`refactor-35`.
This change targets the one rebuilt callable body that still dominates exact `program_lowering`.

### 2. Derive a conservative callable-body partial-rebuild plan

When the changed ranges stay inside one callable body and the runtime can prove safe local
boundaries, exact `program_lowering` should derive a body-local rebuild plan for that callable.

That plan should let the runtime:

- reuse unchanged sibling statement windows inside the callable body;
- rebuild only the changed statement window and any enclosing control-flow region whose boundaries
  are semantically affected by the change;
- avoid recursively dispatching every statement in the callable body solely because one local edit
  occurred inside it.

### 3. Fall back to whole-callable rebuild when boundaries are ambiguous

Body-local rebuild must stay fail-closed.
If the runtime cannot prove safe local boundaries, it must rebuild the whole callable body instead
of guessing.

Examples of conservative whole-callable fallback include, at minimum:

- edits that touch callable signature surfaces such as parameters, export markers, or compiler
  directives;
- edits whose effect crosses unsupported body-local boundaries;
- cases where labels, jumps, exception regions, or other control-flow structure make the local
  rebuild boundary unclear.

### 4. Preserve exact-path orchestration guarantees

Partial callable-body rebuild is not ready if it weakens:

- save-critical promotion for same-version follow-ups;
- latest-wins supersession / retarget toward newer same-file revisions;
- truthful cancellation while exact assembly is in flight;
- fail-closed publication guarantees for exact artifacts.

### 5. Observability must expose direct rebuilt callable-body work

The next representative bundle must answer not just "how long was `program_lowering`?" but also
"how much direct rebuilt callable-body work remained?".

Acceptance evidence should expose, at minimum:

- rebuilt callable count;
- direct rebuilt callable-body dispatch time and call count;
- whole-callable fallback versus bounded body-local rebuild outcome for the traced target;
- residual exact `program_lowering` latency for the same traced target.

Without this, a reduction in parser time would still be under-explained.

## Alternatives Considered

### 1. Start `refactor-36` first

Rejected as the direct continuation of this parser investigation.
`refactor-36` targets semantic diagnostics, but the current parser-side representative residual is
still large and already isolated.
`refactor-36` may still be worthwhile separately, but it does not finish the parser-side job.

### 2. Keep optimizing reuse-plan materialization

Rejected.
The current evidence already shows reuse-plan build, rebase, and reused-progress costs are near
zero on the representative path.

### 3. Jump directly to broad structural sharing

Rejected for now.
It is a much wider ownership/lifetime redesign.
Partial callable-body rebuild is a smaller, better-justified next step with clearer evidence.

## Validation Strategy

- Add parser/runtime regressions showing that a bounded local edit inside one callable body rebuilds
  only the invalidated body-local region when safe.
- Add fail-closed regressions showing that ambiguous body-local boundaries still rebuild the whole
  callable body.
- Preserve save-critical, supersession, retarget, and cancellation regressions on the exact path.
- Export direct rebuilt callable-body observability in the diagnostics-save timeline and
  representative live reports.
- Refresh representative `p53` / `p55` evidence and compare exact `program_lowering` and direct
  rebuilt callable-body work against the `2026-04-17` pre-change trace for this change.

## Quality Gates

- Representative `p55` still publishes through `ready_artifacts`.
- Representative evidence still exports truthful reuse-versus-rebuild counters for exact
  `program_lowering`.
- Representative evidence exports direct rebuilt callable-body metrics for the traced target.
- Representative exact `program_lowering_ms` is materially lower than the `2026-04-17`
  pre-change trace for this parser-side follow-up.
- If the representative parser residual does not materially drop, the change is not ready even if
  synthetic regressions pass.
