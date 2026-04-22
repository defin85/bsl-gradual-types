## ADDED Requirements

### Requirement: Diagnostics-only semantic simplification MUST NOT regress later LSP exact consumers on the same current revision

The system MUST preserve canonical current-revision exact semantics for LSP exact consumers after a
diagnostics-only semantic path has already executed for that same revision.

At minimum this requirement applies to:

- `textDocument/hover`;
- `textDocument/definition`;
- and any other LSP semantic query that shares their exact-only runtime path in the final
  implementation.

This behavior MUST:

- keep diagnostics-only artifacts non-substitutable for exact LSP semantic queries;
- keep later hover/definition requests able to reach the canonical exact artifact for the same
  current revision when that artifact is already ready or becomes ready through the existing
  bounded exact-readiness policy;
- preserve fail-closed empty/unavailable behavior when the exact current-revision artifact is
  genuinely unavailable within bounded policy;
- preserve the current serve-only / fail-closed contract for LSP exact consumers and MUST NOT be
  satisfied by silently re-enabling hidden on-demand exact materialization on the LSP request
  path;
- NOT be satisfied by silently widening diagnostics-only materialization until it effectively
  becomes a second exact contract;
- preserve bounded fail-closed reason-code observability for genuine exact misses.

#### Scenario: Same-revision hover and definition still recover canonical exact semantics after diagnostics-only path

- **GIVEN** a diagnostics-only semantic path has already run for the current document revision
- **AND** a later LSP hover or goto-definition request needs canonical exact semantics for that
  same revision
- **AND** the exact artifact for that revision is already ready or becomes ready through the
  existing bounded exact-readiness policy
- **WHEN** the runtime serves the LSP request
- **THEN** it serves the request from the canonical exact artifact path for that revision
- **AND** it does not treat the diagnostics-only artifact as a successful exact cache hit
- **AND** hover/definition return the expected exact result

#### Scenario: Genuine exact miss remains fail-closed after diagnostics-only path

- **GIVEN** a diagnostics-only semantic path has already run for the current document revision
- **AND** the exact current-revision artifact is still genuinely unavailable within bounded policy
- **WHEN** LSP hover or goto-definition is requested
- **THEN** the response remains empty or unavailable according to the API contract
- **AND** the runtime does not rescue the request with stale, search-only, or diagnostics-only
  semantic substitutes
