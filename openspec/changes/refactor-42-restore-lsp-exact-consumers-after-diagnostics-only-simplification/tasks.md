## 1. Contract

- [ ] 1.1 Define the broken contract precisely: LSP exact consumers must still read or build
      canonical current-revision exact semantics after diagnostics-only simplification without
      reintroducing hidden on-demand exact materialization on the LSP request path.
- [ ] 1.2 Preserve the rule that diagnostics-only artifacts are never substitutes for hover,
      definition, `signatureHelp`, or other exact-only semantic queries.

## 2. Implementation

- [ ] 2.1 Identify where the current diagnostics-only simplification or its runtime wiring breaks
      same-revision exact-consumer recovery for LSP hover/F12.
- [ ] 2.2 Restore the exact runtime path for `hover` and `definition` without widening the
      diagnostics-only artifact into a silent exact substitute and without weakening the existing
      serve-only / fail-closed exact-readiness policy.
- [ ] 2.3 If the same broken boundary also affects `signatureHelp` or `type-at-position`, restore
      that shared exact-only family in the same change.
- [ ] 2.4 Preserve bounded fail-closed behavior and reason-code observability when exact
      current-revision artifacts are genuinely unavailable.

## 3. Regressions and evidence

- [ ] 3.1 Add analysis/runtime regressions proving diagnostics-only isolation still holds after the
      fix.
- [ ] 3.2 Add direct backend/LSP regressions for same-revision hover and goto-definition after a
      diagnostics-only query or equivalent narrowed-path setup, covering the real
      `prepare_lsp_stateful_operation_v2` + handler fail-closed/request-path behavior rather than
      only helper-level analysis tests.
- [ ] 3.3 Preserve or extend fail-closed regressions so exact misses still return empty/unavailable
      results rather than stale substitutes.

## 4. Validation

- [ ] 4.1 Run targeted analysis/runtime/backend tests for diagnostics-only isolation plus restored
      LSP exact behavior.
- [ ] 4.2 Run `openspec validate
      refactor-42-restore-lsp-exact-consumers-after-diagnostics-only-simplification --strict
      --no-interactive`.
