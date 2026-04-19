## ADDED Requirements

### Requirement: Incident bundle MUST preserve diagnostics-save timeout-leaf fidelity from authoritative traces

The extension MUST preserve low-cardinality diagnostics-save timeout-leaf facts in derived
observability incident outputs whenever the authoritative diagnostics-save timeline trace already
contains those facts.

This projection MUST:

- preserve `followup_ready_snapshot_timeout_leaf` and its elapsed fact when the connected server
  contract exposes them;
- keep those fields aligned with the corresponding timeout phase and timeout checkpoint facts from
  the same authoritative request trace;
- fail closed on older server contracts by marking the field unavailable by design, rather than
  silently erasing it on supported contracts.

#### Scenario: Derived bundle preserves `parser_base_recovery` timeout leaf

- **GIVEN** authoritative diagnostics-save timeline trace contains
  `followup_ready_snapshot_timeout_phase=parse_exec`
- **AND** the same trace contains `followup_ready_snapshot_timeout_leaf=parser_base_recovery`
- **WHEN** the extension exports `incident.json` and `summary.md`
- **THEN** the derived request-centric diagnostics-save projection preserves that timeout leaf and
  its elapsed fact
- **AND** operator-facing output does not require opening raw attachments to learn the decisive
  timeout leaf
