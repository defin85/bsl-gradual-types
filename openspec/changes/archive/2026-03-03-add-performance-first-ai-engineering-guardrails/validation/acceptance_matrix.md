# Acceptance Matrix: add-performance-first-ai-engineering-guardrails

Date: March 2, 2026

| Scope | Check | Pass criteria | Fail criteria | Evidence |
| --- | --- | --- | --- | --- |
| Governance | `check-openspec-change-governance.py` | Gate exits `0` and validates `change_criticality`, `test_first_evidence`, ADR/doc-first artifacts, bootstrap policy, rollout dependencies, ownership sign-off | Missing/invalid governance artifact or invalid evidence refs | `openspec/changes/add-performance-first-ai-engineering-guardrails/governance/ownership_signoff.json` |
| OpenSpec | `openspec validate ... --strict --no-interactive` | Change validates strictly with no errors | Any strict validation failure | `openspec/changes/add-performance-first-ai-engineering-guardrails/validation/final-closure-checklist.md` |
| Protected assets | `check-protected-assets-gate.py` | No protected changes without approved override; override path validated | Protected assets changed without override or invalid override payload | `openspec/changes/add-performance-first-ai-engineering-guardrails/governance/protected_assets_override.json` |
| Contracts | `check-versioned-contracts.py` + compatibility diff | Versioned contract surface valid; no breaking diff without major bump/migration note | Contract schema/policy violation or breaking diff without required versioning | `contracts/intellisense-perf-gate/v1/contract.json` |
| Option B boundary | `check-perf-gate-architecture.py` | No inline verdict/reason-code logic outside dedicated evaluator | Any architecture boundary violation for perf evaluator | `openspec/changes/add-performance-first-ai-engineering-guardrails/validation/perf-gate-architecture-boundary.md` |
| Perf gate | `scripts/run-intellisense-perf.sh` (`small|large|churn`) | `verdict=pass`, empty `reason_codes`, contract version `v1` | Non-pass verdict or non-empty blocking reason-codes | `openspec/changes/add-performance-first-ai-engineering-guardrails/validation/perf-gate-dry-run-small-large-churn.md` |
