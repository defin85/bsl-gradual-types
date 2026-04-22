## ADDED Requirements

### Requirement: Exact same-version `program_lowering` MUST materialize safe reuse without a second deep-clone of unchanged regions

The system MUST, when applying a conservative exact same-version lowering-reuse plan for the
current ready-snapshot target, materialize reused top-level lowering units and reused callable-body
statement windows by ownership transfer or an equivalently bounded no-extra-clone path rather than
deep-cloning the unchanged subtree a second time before final `Program` assembly.

This behavior MUST:

- preserve the fail-closed invalidation boundaries introduced by
  `refactor-33-exact-program-lowering-changed-range-reuse`;
- preserve exact same-version semantics, latest-wins supersession, and truthful
  cancellation/retarget behavior for save-follow-up exact assembly;
- remove the second full-subtree deep-clone during final `Program` assembly for reused regions;
- allow at most one bounded rebase/update pass needed to align moved reused nodes to the current
  revision, rather than silently expanding this change into a broader structural-sharing rewrite.

#### Scenario: Local same-file edit reuses unchanged lowered regions without a second deep clone

- **GIVEN** the previous exact ready snapshot already proved many top-level lowering units and
  callable-body statement windows unchanged
- **AND** the current same-file target still qualifies for conservative reuse under the existing
  invalidation rules
- **WHEN** exact `program_lowering` materializes the final `Program`
- **THEN** the runtime moves those unchanged regions into the final assembly through the consumed
  reuse plan
- **AND** it does not deep-clone the full reused subtree a second time solely to rebuild the final
  `Program`

#### Scenario: Ambiguous invalidation still rebuilds instead of reusing

- **GIVEN** a same-file edit touches or may affect a lowering boundary whose reuse soundness is not
  proven
- **WHEN** exact `program_lowering` derives or applies the reuse plan
- **THEN** the affected region is rebuilt fail-closed
- **AND** the runtime does not use ownership-based materialization to bypass rebuild eligibility

### Requirement: Exact reuse observability MUST remain truthful after ownership-based plan consumption

The system MUST preserve truthful reuse-versus-rebuild attribution for one traced exact same-file
save-follow-up target even when the lowering-reuse plan is consumed by ownership during final
`program_lowering` assembly.

This evidence MUST include at least:

- the reuse-plan outcome for the traced exact target;
- bounded reused-versus-rebuilt lowering workload counts;
- the residual exact `program_lowering` latency for that same traced target.

#### Scenario: Representative follow-up still explains reduced exact lowering work truthfully

- **GIVEN** ownership-based reuse materialization is enabled for a representative same-file
  save-follow-up target
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the report still exposes both the exact `program_lowering` residual and the
  reused-versus-rebuilt lowering breakdown for that traced target
- **AND** operators can distinguish "less work was materialized" from "the same work was merely
  relabeled"
