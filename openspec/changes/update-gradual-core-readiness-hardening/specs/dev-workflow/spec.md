## ADDED Requirements

### Requirement: Readiness gate derives verdict from referenced evidence artifacts (MUST)
If a change uses machine-readable readiness governance, the readiness gate MUST treat the artifacts referenced by `review_ref` and `traceability_ref` as authoritative evidence for review and traceability verdicts.

`governance/readiness_status.json` MUST NOT be allowed to override conflicting referenced evidence by self-reporting `review_verdict` or `traceability_status`.

If the referenced artifact is missing a canonical verdict/status or cannot be parsed deterministically, the gate MUST fail closed.

#### Scenario: Declared review success conflicts with referenced review artifact
- **GIVEN** `readiness_status.json` declares successful review status
- **AND** `review_ref` points to an approved repository artifact whose content indicates `partial`, `gap`, or equivalent unresolved state
- **WHEN** the readiness gate validates the change
- **THEN** the gate rejects the optimistic verdict
- **AND** the change cannot be treated as `complete`

#### Scenario: Declared traceability success conflicts with referenced traceability artifact
- **GIVEN** `readiness_status.json` declares successful traceability status
- **AND** `traceability_ref` points to an approved repository artifact whose content indicates unresolved coverage or gap
- **WHEN** the readiness gate validates the change
- **THEN** the gate rejects the optimistic verdict
- **AND** the change cannot be treated as `complete`

### Requirement: Superseding delivery path is explicit, approved, and regression-tested (MUST)
If `declared_status=complete` is allowed despite open critical backlog, the gate MUST require an explicit approved `superseding_delivery_path` and MUST validate it fail-closed.

The referenced superseding artifact MUST:
- live in the repository;
- be scoped to the intended change root;
- contain explicit approval/handoff evidence for the replacing delivery path.

#### Scenario: Approved superseding delivery path allows complete despite open backlog
- **GIVEN** a change still has open critical backlog
- **AND** `readiness_status.json` declares `complete`
- **AND** `superseding_delivery_path` points to approved replacing delivery evidence
- **WHEN** the readiness gate validates the change
- **THEN** the gate may accept `complete`
- **AND** the decision is explained by the approved superseding path rather than by the open backlog state itself

#### Scenario: Invalid superseding delivery path fails closed
- **GIVEN** a change still has open critical backlog
- **AND** `superseding_delivery_path` is missing, out of scope, or not explicitly approved
- **WHEN** the readiness gate validates the change
- **THEN** the gate rejects `declared_status=complete`
- **AND** the change remains blocked by the open backlog
