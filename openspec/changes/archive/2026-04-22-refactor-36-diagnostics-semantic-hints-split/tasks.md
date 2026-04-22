## 1. Diagnostics-only semantic contract

- [x] 1.1 Define a dedicated diagnostics type-hints artifact that includes only the hint maps
      consumed by semantic diagnostics.
- [x] 1.2 Preserve diagnostics parity while full `SemanticFacts` remain the contract for
      interactive exact features and other non-diagnostics consumers.

## 2. Diagnostics materialization split

- [x] 2.1 Wire semantic diagnostics to build diagnostics-only type hints without full
      `SemanticFacts` materialization when the request does not need the full artifact.
- [x] 2.2 Remove diagnostics-irrelevant work from the diagnostics-only path where soundness is
      proven, including incomplete-member-access recovery on syntactically valid non-interactive
      diagnostics targets if representative evidence confirms it is unnecessary there.

## 3. Cache isolation and observability

- [x] 3.1 Ensure diagnostics-only artifacts are ephemeral or stored under a separate diagnostics
      cache namespace, never under the current full exact semantic cache key.
- [x] 3.2 Export low-cardinality observability distinguishing diagnostics-only hint materialization
      from full semantic-facts materialization for representative save-follow-up evidence.

## 4. Evidence and regressions

- [x] 4.1 Add parity regressions comparing semantic diagnostics output between the full path and the
      diagnostics-only hints path on representative modules.
- [x] 4.2 Add cache-isolation regressions proving diagnostics-only queries cannot poison later
      completion, hover, definition, `signatureHelp`, or type-at-position requests.
- [x] 4.3 Refresh representative `p55` live evidence and compare the diagnostics residual against
      the latest post-parser-side baseline.

## 5. Validation

- [x] 5.1 Run targeted `analysis-v2`, `semantic-diagnostics`, and backend tests covering
      diagnostics parity, cache isolation, and representative save-follow-up behavior.
- [x] 5.2 Run `openspec validate refactor-36-diagnostics-semantic-hints-split --strict
      --no-interactive`.
