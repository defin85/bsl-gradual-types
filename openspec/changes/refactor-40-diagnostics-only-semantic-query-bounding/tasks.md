## 1. Contract

- [ ] 1.1 Define the representative diagnostics-only semantic-query contract for the same-file
      save-follow-up family after `refactor-39` exact-path stabilization.
- [ ] 1.2 Preserve truthful diagnostics-only vs full-semantic-facts fallback semantics while
      reducing the residual, so the change cannot pass by masking work under another path.

## 2. Implementation

- [ ] 2.1 Use the leaf attribution exposed by
      `refactor-38-diagnostics-only-semantic-facts-leaf-profiling` to identify and bound the
      dominant diagnostics-only semantic-query residual on the representative path, with
      diagnostics-only semantic-facts build treated as the default first branch unless the new
      truthful leaf profile disproves it.
- [ ] 2.2 Rework the diagnostics-only facts-build branch so the representative path no longer pays
      for full diagnostics-only `SemanticFacts` output that is immediately collapsed away before
      `SemanticValidationVisitor` runs, while preserving the same four observed hint maps and
      without regressing `ready_artifacts` incidence.
- [ ] 2.3 If refreshed truthful diagnostics-only leaf evidence still shows a dominant builder
      sub-leaf after the reduced-output branch lands, bound that sub-leaf next
      (`seed_module_context`, `local_function_summaries`, or statement/body visitation) with
      parity proof for the four-map hint surface and downstream diagnostics.
- [ ] 2.4 Revisit diagnostics collection only if refreshed truthful leaf evidence shows
      `collect_ms` remains dominant after the reduced-output builder branch is bounded.
- [ ] 2.5 Preserve truthful observability/report wiring so representative `p55` leaf drilldown and
      `p56` family evidence remain directly comparable to the checked-in `refactor-39` baseline.

## 3. Regressions and evidence

- [ ] 3.1 Add targeted `analysis-v2` and backend regressions covering reduced-output
      diagnostics-only facts/hints parity, downstream diagnostics parity, and truthful full-path
      fallback when the optimization is not proven.
- [ ] 3.2 Refresh representative `p55` leaf evidence and representative `p56` family evidence
      against the checked-in `refactor-39` bundle.

## 4. Validation

- [ ] 4.1 Run targeted diagnostics semantic tests and representative live repros for the new
      diagnostics-only query-bounding contract.
- [ ] 4.2 Run `openspec validate refactor-40-diagnostics-only-semantic-query-bounding --strict
      --no-interactive`.
