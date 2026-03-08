# Design: update-gradual-core-readiness-hardening

## Context
The prior change `update-gradual-core-production-readiness` brought the gradual-typing structural contract close to production readiness, but review-only verification found one remaining process gap and one bounded semantic exception.

What is already strong:
- shared structural members are first-class on `TypeResolution`;
- exact cross-consumer acceptance covers typed `Структура` and typed-row scenarios;
- the change-specific governance gate already blocks `complete` on open critical backlog.

What is still weak:
- the gate trusts declared `review_verdict` / `traceability_status` instead of validating the referenced artifacts themselves;
- `superseding_delivery_path` is supported in code but not pinned by explicit regression coverage;
- completion still contains a bootstrap-only implicit module-context fallback that is bounded by design but not yet fully retired or proven by direct evidence.

## Goals
- Make readiness verdicts fail-closed against referenced evidence, not only against self-reported JSON fields.
- Define a deterministic policy for `superseding_delivery_path`.
- Remove or directly prove the last bounded completion fallback so it cannot drift into a second semantic truth.

## Non-Goals
- Do not redesign the shared structural contract itself.
- Do not broaden the acceptance scope beyond the remaining readiness and fallback gaps.
- Do not reopen unrelated parts of `update-gradual-core-production-readiness`.

## Decisions

### 1. Readiness verdict must be evidence-derived
`governance/readiness_status.json` remains the machine-readable declaration surface, but it MUST NOT be treated as the primary source of truth for review or traceability verdicts.

The gate will treat `review_ref` and `traceability_ref` as authoritative evidence sources and will either:
- derive canonical verdict/status from those artifacts; or
- fail closed if the artifacts do not expose a parseable canonical verdict/status.

Self-reported JSON fields are only valid if they match the referenced evidence.

### 2. Superseding delivery path must be explicit and testable
An open critical backlog may coexist with `declared_status=complete` only when:
- `superseding_delivery_path` points inside the repository and inside the intended change root;
- the referenced artifact contains explicit approval/handoff evidence;
- the artifact points to a real replacing delivery path, not generic prose.

The positive path and the rejection paths must both be regression-tested.

### 3. Bootstrap-only implicit module-context fallback needs a hard boundary
The remaining fallback in completion member resolution is acceptable only as a transitional bootstrap path for implicit module-context symbols.

End-state options for this change:
- preferred: remove the fallback and converge on the shared owner-hint path end-to-end;
- acceptable only with direct evidence: keep the fallback narrowly bounded and prove by automated tests that it does not create a second structural truth and does not weaken reviewed fail-closed scenarios.

## Risks / Trade-offs
- Hardening the gate may invalidate existing optimistic artifacts.
  - Mitigation: closure artifact refresh is part of the same change.
- Removing the fallback entirely may require wider runtime changes than expected.
  - Mitigation: the contract allows a bounded-evidence end-state if the shared truth invariant is proven directly.

## Migration Plan
1. Add failing governance and fallback-boundary tests.
2. Harden the gate and runtime path until tests pass.
3. Refresh `update-gradual-core-production-readiness` artifacts to match the hardened contract.
4. Validate the new change strictly before handoff.

## Open Questions
- None. The remaining ambiguity is operational, not product-level.
