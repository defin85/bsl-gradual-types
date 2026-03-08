# Acceptance Matrix: update-gradual-core-production-readiness

Date: March 8, 2026

| Scope | Check | Pass criteria | Fail criteria | Evidence |
| --- | --- | --- | --- | --- |
| OpenSpec | `openspec validate update-gradual-core-production-readiness --strict --no-interactive` | Change validates strictly with no errors | Any strict validation failure in change files or specs | `openspec/changes/update-gradual-core-production-readiness/validation/final-closure-checklist.md` |
| Governance | `python3 scripts/check-openspec-change-governance.py --change-id update-gradual-core-production-readiness` | Gate exits `0`; required governance artefacts exist; `declared_status` is consistent with review/traceability and Beads backlog | Missing governance artefact, invalid schema, optimistic `complete`, or broken readiness/backlog alignment | `openspec/changes/update-gradual-core-production-readiness/governance/readiness_status.json` |
| Semantic residual risks | Production-readiness residual risk review | P1/P2 semantic edge-case risks are retired by stronger shared-contract guarantees or mapped to focused automated evidence | Any residual risk remains unowned, untested, or justified only by prose | `openspec/changes/update-gradual-core-production-readiness/residual-risk-review.md` |
| Ownership | Role-based sign-off review | `analysis_v2`, `runtime`, `LSP`, and `process` ownership evidence is recorded and references concrete change-local artefacts | Missing required role, missing evidence ref, or sign-off not approved | `openspec/changes/update-gradual-core-production-readiness/validation/review-ownership-signoff.md` |
| Final readiness honesty | Current status review | Final verdict is `complete` only after traceability refresh, closure evidence refresh, and closure of the critical backlog listed in `readiness_status.json` | Any wording or status implies `complete` while closure evidence or listed critical backlog is still open | `openspec/changes/update-gradual-core-production-readiness/validation/readiness-review-status.md` |
