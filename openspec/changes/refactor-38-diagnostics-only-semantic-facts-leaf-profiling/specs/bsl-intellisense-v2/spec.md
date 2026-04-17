## ADDED Requirements

### Requirement: Diagnostics-only semantic evidence MUST export path-specific leaf attribution

When semantic diagnostics use the diagnostics-only materialization path, the system MUST export
path-specific leaf attribution for the diagnostics-only semantic-facts builder rather than only an
aggregate diagnostics-only IR total.

At minimum this evidence MUST distinguish:

- AST->IR conversion time;
- diagnostics-only semantic-facts build subphases that actually ran for the traced target;
- diagnostics collection time after diagnostics-only materialization;
- the traced diagnostics semantic materialization path for that target;
- the absence of full-semantic-facts-only subphases that did not run on that path.

The diagnostics-only leaf surface MUST use a dedicated diagnostics-only field family or equivalent
dedicated namespace.

Reusing the existing `semantic_diagnostics_ir_semantic_facts_*` full-path field family for
diagnostics-only work MUST NOT satisfy this requirement, even if the old fields are accompanied by
best-effort comments or indirect cumulative metrics.

The exported diagnostics-only leaf attribution MUST be sourced from the diagnostics-only builder
profile returned by `analysis-v2` rather than heuristically reconstructed only downstream.

#### Scenario: Representative report explains the diagnostics-only residual truthfully

- **GIVEN** a representative same-file save-follow-up uses diagnostics-only semantic
  materialization
- **WHEN** the runtime exports the traced diagnostics report
- **THEN** the report includes diagnostics-only leaf attribution for that traced target
- **AND** the report includes the traced diagnostics semantic `materialization_path`
- **AND** skipped full-semantic-facts-only subphases stay absent or zero
- **AND** operators can see whether the remaining residual is in AST->IR, diagnostics-only facts
  build, or diagnostics collection

#### Scenario: Reusing the old full-path leaf family without traced path identity is rejected

- **GIVEN** an implementation exports diagnostics-only timings only through the old
  `semantic_diagnostics_ir_semantic_facts_*` field family or omits the traced
  `materialization_path`
- **WHEN** the representative diagnostics report is reviewed
- **THEN** the requirement is not satisfied
- **AND** the diagnostics-only leaf surface is still considered ambiguous
