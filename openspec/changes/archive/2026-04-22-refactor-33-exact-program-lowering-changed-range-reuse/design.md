## Context

The latest representative bundle on `git c172fe76` changed the problem statement again:

1. `didChange` no longer falls back through `stale_parser_base`;
2. `didSave` heavy follow-up now returns through `ready_artifacts` instead of `shadow_state`;
3. completion transport and extension pre-send are still not the bottleneck;
4. the dominant exact-path residual remains `program_lowering` at about `2.57s`.

That means the next change must reduce real exact-path CPU work.
More probes, more waiting, or more routing tweaks would not address the dominant cost already shown
by the new bundle.

## Goals / Non-Goals

- Goals:
  - reduce exact `program_lowering` work for local same-file edits on representative large-module
    profiles;
  - preserve exact same-version semantics and latest-wins supersession;
  - make representative bundles show how much lowering was reused versus rebuilt.
- Non-Goals:
  - broaden bounded waits;
  - weaken invalidation boundaries;
  - treat UI / transport as the main suspect for this incident class;
  - solve unrelated cold-start `save_fastlane` behavior in the same change.

## Decisions

### 1. Optimize lowering work, not just lowering visibility

`refactor-31` already introduced cooperative checkpoints inside exact lowering.
`refactor-32` already improved exact-head freshness enough to avoid the old fallback path.
The new residual therefore points to actual lowering cost.

`refactor-33` should target "less exact lowering work for local edits", not merely "more chances to
observe the same full lowering span".

### 2. Derive a conservative `LoweringReusePlan` from previous exact artifacts plus changed ranges

The exact path already has access to:

- the previous ready parse snapshot;
- the old exact `ParseResult`;
- new tree-sitter changed ranges;
- current same-file text and version.

The new design should derive a reuse plan before exact assembly starts.
That plan should answer:

- which top-level lowered units are definitely unchanged and reusable;
- which enclosing callable bodies must be rebuilt;
- whether a body-local rebuild can reuse unchanged sibling statements / windows;
- whether ambiguity forces a full rebuild of the affected region.

### 3. Reuse must stay fail-closed and invalidation-driven

Safe reuse is more important than aggressive reuse.

The runtime MAY reuse unchanged lowered units only when it can prove the edit did not invalidate
their lowering result.
The runtime MUST rebuild instead of reuse when an edit touches or may affect:

- routine headers or parameter lists;
- local declaration blocks whose bindings affect later statements;
- branch / loop / exception headers that change control-flow structure;
- spans where changed ranges cannot be mapped to stable old lowering units;
- any future construct whose reuse soundness is not yet proven.

### 4. Reuse should be incremental at two levels

The first implementation pass should support:

- top-level reuse of unchanged declarations / routines;
- body-local reuse of unchanged sibling statement windows inside one rebuilt routine.

This is narrower than full AST diffing and easier to validate against exactness.
It is also more likely to address the representative `conf_big` case, where one local edit inside a
large procedure currently appears to trigger too much full-body lowering.

### 5. Observability must expose reused versus rebuilt work

The next bundle should be able to answer not only "how long was `program_lowering`?" but also
"what fraction of lowering was reused versus rebuilt?"

The implementation should therefore expose metrics / trace fields such as:

- reused lowering unit count;
- rebuilt lowering unit count;
- reused window count;
- rebuilt window count;
- largest rebuilt window size;
- reuse-plan outcome (`full_rebuild`, `top_level_reuse`, `body_local_reuse`, etc.).

### 6. Keep a kill switch for the new reuse path

Because this change modifies exact-path semantics internally, the new reuse path should be guarded
by a runtime config switch during rollout.
The default may still be enabled for validation, but rollback should not require reverting the
entire change under production pressure.

## Alternatives Considered

### 1. Add more checkpoints without reducing work

Rejected.
That would improve responsiveness to cancellation / promotion but would not materially change the
`2.57s` dominant residual already proven by the new bundle.

### 2. Reuse only the current append-style prefix fast path

Rejected.
The current narrow prefix reuse is already present and is not enough for the representative local
same-file churn profile.

### 3. Introduce aggressive whole-body memoization

Rejected for now.
The invalidation surface is wider and harder to prove sound.
Changed-range-aware conservative reuse is a smaller and more defensible first step.

## Validation Strategy

- Add parser/runtime regressions proving that local edits reuse unchanged lowering units while
  preserving exactness.
- Add regressions proving that ambiguous invalidation falls back to rebuild instead of reuse.
- Add regressions proving that supersession / retarget still work while reused-lowering batches are
  in flight.
- Refresh representative `conf_big` live evidence and compare it to the `c172fe76` baseline.

## Quality Gates

- Representative bundle still shows:
  - `followup_semantic_path=ready_artifacts`
  - `intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_stale_parser_base=0`
- Representative bundle shows reduced exact `program_lowering_ms` versus the `c172fe76` baseline.
- Representative bundle or checked-in evidence explains the reduction using reused-versus-rebuilt
  lowering work, not only wall-clock timing.
- If the reduction is not material, the change is not ready even if synthetic regressions pass.
