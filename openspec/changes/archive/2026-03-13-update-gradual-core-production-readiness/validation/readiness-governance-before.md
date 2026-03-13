# Before remediation: readiness governance failed closed

Date: March 8, 2026

## Status
fail

## Observed failure / regression signals

- Before `6mx.4`, change-local governance artefacts were missing.
- `governance/change_criticality.json` did not exist.
- `governance/readiness_status.json` did not exist.
- `tasks.md` had no machine-readable `D*` dependency lines for dependency checks.
- No acceptance matrix existed for the architectural change.

## Why this was a problem

Before remediation the change could pass `openspec validate`, but readiness governance still failed
closed because the project had no machine-readable way to reject optimistic `complete` claims.

Reason_codes:
- `missing_governance_artifacts`
- `optimistic_readiness_without_gate`
