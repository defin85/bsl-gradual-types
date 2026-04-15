## ADDED Requirements

### Requirement: Exact same-version `program_lowering` MUST reuse unchanged lowering units for local same-file edits when safe

The system MUST derive a conservative lowering-reuse plan for exact same-version ready-snapshot
assembly from the previous exact ready state and the current changed ranges.
When that plan proves that some lowering units are unchanged, the runtime MUST reuse them instead
of rebuilding the entire lowering region.

This behavior MUST:

- stay bound to the exact `(file_id, requested_version, text_hash)` target;
- support reuse of unchanged top-level lowering units and bounded body-local reuse of unchanged
  sibling statement windows when soundness can be proven;
- rebuild any lowering region whose invalidation boundary cannot be proven safely;
- preserve exact same-version semantics, latest-wins supersession, and truthful cancellation /
  retarget behavior.

#### Scenario: Local edit inside one large callable body reuses unchanged lowering units

- **GIVEN** the previous same-file revision already has an exact ready snapshot
- **AND** the new revision changes only a bounded local region inside one large callable body
- **WHEN** exact ready-snapshot assembly builds the still-current target
- **THEN** the runtime reuses unchanged lowering units outside the invalidated region
- **AND** the exact path does not rebuild the whole file or whole body solely because one local
  edit occurred

#### Scenario: Ambiguous invalidation falls back to rebuild instead of stale reuse

- **GIVEN** a same-file edit touches or may affect a lowering boundary whose reuse soundness is not
  proven
- **WHEN** the runtime derives the exact lowering-reuse plan
- **THEN** the affected region is rebuilt fail-closed
- **AND** the system does not publish stale exact artifacts by guessing that reuse is safe

### Requirement: Exact `program_lowering` reuse MUST remain observable on representative load

The system MUST export operator-facing evidence showing how much exact `program_lowering` work was
reused versus rebuilt for one traced target on representative large-module same-file churn.
Acceptance for this change MUST prove reduced exact lowering work rather than only reduced wall-clock
latency with no visibility into what changed.

This behavior MUST:

- keep reuse-versus-rebuild evidence tied to one exact traced target and save cycle;
- expose a truthful reuse-plan outcome for the operator-facing trace or metrics snapshot;
- expose bounded summaries of reused and rebuilt lowering work for the exact path;
- preserve truthful dominant checkpoint and timeout attribution for the same traced target.

#### Scenario: Representative follow-up reports reduced exact lowering work with reuse evidence

- **GIVEN** a representative large-module same-file save follow-up exercises the exact path after
  the lowering-reuse change
- **WHEN** a live diagnostics-save bundle or checked-in report is exported
- **THEN** the evidence shows both the exact `program_lowering` residual and the reuse-versus-rebuild
  breakdown for that traced target
- **AND** operators can distinguish "less work was rebuilt" from "the system merely waited longer
  or relabeled the same hotspot"

#### Scenario: Full rebuild remains truthful when reuse does not qualify

- **GIVEN** the lowering-reuse plan decides that the current exact target must rebuild the affected
  region completely
- **WHEN** observability exports the traced follow-up result
- **THEN** the reuse-versus-rebuild evidence truthfully reports that reuse did not qualify
- **AND** dominant checkpoint and timeout attribution remain coherent for that full-rebuild path
