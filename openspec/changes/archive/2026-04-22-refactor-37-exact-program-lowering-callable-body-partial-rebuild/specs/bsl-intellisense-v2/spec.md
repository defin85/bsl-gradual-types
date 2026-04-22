## ADDED Requirements

### Requirement: Exact same-version `program_lowering` MUST avoid whole-callable body rebuild for bounded local edits when safe

The system MUST, for an exact same-version ready-snapshot target whose changed ranges stay inside
one callable body, derive a conservative callable-body partial-rebuild plan when body-local
invalidation boundaries can be proven safely.

When that plan proves that only a bounded local region inside the callable body is invalidated, the
runtime MUST rebuild only the invalidated statement window and any semantically dependent enclosing
control-flow region, rather than recursively dispatching every statement in the callable body.

This behavior MUST:

- stay bound to the exact `(file_id, requested_version, text_hash)` target;
- preserve the fail-closed invalidation discipline established by
  `refactor-33-exact-program-lowering-changed-range-reuse`;
- preserve exact same-version semantics, latest-wins supersession, and truthful
  cancellation/retarget behavior;
- rebuild the whole callable body instead of guessing when body-local soundness is not proven.

#### Scenario: Bounded local edit inside one large callable body rebuilds only the invalidated body window

- **GIVEN** the previous same-file revision already has an exact ready snapshot
- **AND** the new revision changes only a bounded local region inside one large callable body
- **AND** the runtime can prove safe body-local invalidation boundaries for that edit
- **WHEN** exact ready-snapshot assembly lowers the still-current target
- **THEN** the runtime rebuilds only the invalidated body-local region and any semantically
  dependent enclosing control-flow region
- **AND** it does not recursively dispatch the whole callable body solely because that one local
  edit occurred

#### Scenario: Ambiguous body-local invalidation falls back to whole-callable rebuild

- **GIVEN** a same-file edit inside one callable body touches or may affect a body-local boundary
  whose rebuild soundness is not proven
- **WHEN** exact ready-snapshot assembly derives or applies the callable-body partial-rebuild plan
- **THEN** the affected callable body is rebuilt fail-closed
- **AND** the runtime does not guess a narrower partial-rebuild boundary

### Requirement: Representative exact lowering observability MUST expose rebuilt callable-body work

The system MUST export operator-facing evidence showing how much direct rebuilt callable-body work
remains for one traced exact same-file save-follow-up target on representative large-module churn.

This evidence MUST include at least:

- the exact `program_lowering` residual for the traced target;
- whether the rebuilt callable used bounded body-local rebuild or whole-callable fallback;
- direct rebuilt callable-body dispatch time and call count for that traced target.

#### Scenario: Representative follow-up explains parser residual using rebuilt callable-body metrics

- **GIVEN** a representative large-module same-file save follow-up exercises the exact path after
  the callable-body partial-rebuild change
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the evidence shows the exact `program_lowering` residual and the direct rebuilt
  callable-body metrics for that traced target
- **AND** operators can distinguish "less callable-body work was rebuilt" from "the same parser
  hotspot was only relabeled"

#### Scenario: Whole-callable fallback remains truthful in representative evidence

- **GIVEN** the traced exact target falls back to whole-callable rebuild because body-local
  boundaries are ambiguous
- **WHEN** observability exports the representative follow-up result
- **THEN** the report truthfully indicates that bounded callable-body partial rebuild did not
  qualify
- **AND** the direct rebuilt callable-body metrics remain coherent for that fallback path
