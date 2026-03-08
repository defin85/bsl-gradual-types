# Change: update-gradual-core-readiness-hardening

## Why
Review-only verification of `update-gradual-core-production-readiness` found that the semantic part of the change is largely delivered, but the final readiness contract is still not fully fail-closed.

The remaining gaps are:
- the change-specific readiness gate still trusts self-reported `review_verdict` and `traceability_status` from `governance/readiness_status.json` instead of validating the referenced evidence content;
- regression coverage does not yet pin down the positive/negative policy around `superseding_delivery_path`;
- a bounded bootstrap-only implicit module-context fallback still exists in completion member resolution and is documented mostly by prose instead of direct automated evidence or removal.

## What Changes
- Harden the `dev-workflow` readiness gate so declared readiness cannot override conflicting review/traceability artifacts.
- Define and validate the exact policy for approved `superseding_delivery_path` when critical backlog is still open.
- Prove or remove the remaining bootstrap-only implicit module-context fallback in completion member resolution so it cannot survive as hidden semantic truth.
- Refresh change-local closure artifacts for `update-gradual-core-production-readiness` after the hardened gate lands.

## Impact
- Affected specs:
  - `dev-workflow`
  - `bsl-intellisense-v2`
- Affected code:
  - `scripts/check-openspec-change-governance.py`
  - `scripts/test-openspec-change-governance.py`
  - `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service/tests.rs`
  - `openspec/changes/update-gradual-core-production-readiness/**`

## Related Execution Graph
- Beads epic: `bsl-gradual-types-rri`
- This change is the OpenSpec contract for the remaining work tracked under that epic.
