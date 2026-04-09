## 1. Implementation
- [ ] 1.1 Introduce reusable immutable non-member completion catalogs for deps-scoped candidate families such as global functions, metadata items, repository types, and keywords, keyed by deps/settings snapshot or a semantically equivalent immutable snapshot identity.
- [ ] 1.2 Apply prefix-aware filtering before full `Candidate` materialization for immutable deps-scoped families while preserving existing result correctness and source-priority semantics.
- [ ] 1.3 Keep local/contextual/module-routine candidate collection revision-sensitive and separate from the new immutable catalog path.
- [ ] 1.4 Export dedicated collect-stage evidence so operator-facing reports can attribute warm non-member latency to specific source families before ranking/formatting.

## 2. Validation
- [ ] 2.1 Add deterministic regressions proving warm non-member completion no longer rebuilds immutable deps-wide candidate families on every request.
- [ ] 2.2 Add correctness regressions proving lexical local symbols and context-sensitive candidates remain unchanged by the immutable catalog path.
- [ ] 2.3 Add representative perf/live acceptance covering warm non-member collect budgets and source-family breakdowns.
- [ ] 2.4 Re-run existing non-member completion regressions to confirm no ranking/result regressions.
- [ ] 2.5 Run `openspec validate refactor-13-non-member-completion-catalog-precompute --strict --no-interactive`.
