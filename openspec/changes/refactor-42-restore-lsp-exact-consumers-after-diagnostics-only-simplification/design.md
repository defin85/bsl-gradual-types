## Context

User-visible evidence now says that `hover` and F12 (`textDocument/definition`) regressed after the
recent simplification work.

The nearby accepted contract already says:

- diagnostics-only semantic hints are a narrower artifact used only for semantic diagnostics;
- full `SemanticFacts` remain the contract for interactive exact features;
- diagnostics-only artifacts must not poison later exact consumers.

The current codebase also makes the likely failure mode specific:

- `analysis-v2/src/type_inference_v2/tests.rs` explicitly asserts that diagnostics-only build does
  not materialize exact call targets, member targets, constructor targets, or definition
  locations;
- `analysis-v2/src/lib/tests.rs` asserts that diagnostics-only path does not publish completion
  head or exact type-index readiness by accident;
- backend tests already assert fail-closed behavior when canonical exact truth is genuinely
  missing;
- but current acceptance does not directly prove that a same-revision LSP hover/F12 request still
  succeeds after diagnostics-only simplification when exact canonical artifacts are in fact
  buildable and expected to work.

So the current acceptance surface can miss a regression where:

- diagnostics-only simplification remains locally correct for diagnostics;
- exact artifacts are not explicitly poisoned in the analysis-layer tests;
- yet the LSP/runtime integration path for hover/definition no longer recovers the exact artifact
  correctly for the current revision.

This change is therefore about restoring the end-to-end exact-consumer contract, not about growing
the diagnostics-only artifact.

## Goals / Non-Goals

- Goals:
  - restore correct LSP behavior for hover and goto-definition after diagnostics-only
    simplification;
  - preserve the architectural rule that diagnostics-only artifacts are not substitutes for full
    exact semantics;
  - close the acceptance gap by adding direct LSP/runtime regressions for same-revision exact
    consumers after narrowed-path work;
  - preserve fail-closed behavior when exact current-revision artifacts are actually unavailable.
- Non-Goals:
  - make diagnostics-only artifact rich enough to answer hover/definition directly;
  - reopen diagnostics latency work from `refactor-40`;
  - weaken fail-closed semantics or allow stale semantic substitutes;
  - treat MCP/web parity or broader GA readiness as a substitute for fixing the broken LSP exact
    contract.

## Decisions

### 1. Restore the exact path instead of expanding the diagnostics-only artifact

The diagnostics-only artifact intentionally excludes exact targets and definition locations.
That is not a bug by itself; it is part of the design.

So the correct fix is not "put more exact data back into diagnostics-only until hover/F12 happen
to work again". The correct fix is to ensure exact LSP consumers still reach the canonical exact
artifact path when they need it.

### 2. Treat this as an end-to-end LSP/runtime contract regression

Analysis-layer isolation tests are necessary, but they are not sufficient.

The regression is user-facing and LSP-specific, so acceptance must include the LSP request path
itself: current revision state, narrowed diagnostics-only work, later hover/F12 request, and the
expected exact response.

### 3. Preserve fail-closed semantics for genuine misses

Restoring hover/F12 must not come from allowing diagnostics-only, search/discovery, or stale
artifacts to masquerade as exact truth.

If current exact artifacts are genuinely unavailable within the bounded policy, hover/definition
must remain empty/unavailable with the same bounded fail-closed reasons.

### 4. Cover the whole exact-only family if they share the same broken path

The user-reported regression names hover and definition. Those are the minimum acceptance
surfaces.

If implementation shows that `signatureHelp` or `type-at-position` share the same broken exact
runtime boundary, the fix must cover them in the same change rather than leaving another obvious
regression behind.

## Alternatives Considered

### 1. Treat this as “just a missing test”

Rejected.

The user-facing regression means the contract is already broken in behavior, not only in coverage.
New tests are necessary but not sufficient.

### 2. Put definition locations and exact targets back into diagnostics-only materialization

Rejected.

That would blur the boundary that `refactor-36` explicitly introduced and risks recreating the same
coupling under a different name.

### 3. Allow search/discovery or other non-exact artifacts to rescue hover/F12

Rejected.

Canonical exact queries in `bsl-intellisense-v2` are already specified as fail-closed when exact
current-revision artifacts are unavailable.

## Validation Strategy

- Add direct backend/LSP regressions proving that same-revision hover and goto-definition still
  succeed after diagnostics-only simplification when exact semantics are available.
- Preserve existing analysis-layer isolation regressions showing diagnostics-only artifacts do not
  publish exact readiness accidentally.
- Preserve fail-closed regressions showing hover/definition stay empty when exact current-revision
  artifacts are genuinely unavailable.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- Hover and goto-definition work again on the restored exact path.
- The fix does not rely on diagnostics-only artifacts becoming substitutes for full exact
  semantics.
- Existing fail-closed guarantees remain intact for genuine exact misses.
- If hover/F12 only start “working” by reading stale or narrowed diagnostics artifacts, the change
  is not ready.
