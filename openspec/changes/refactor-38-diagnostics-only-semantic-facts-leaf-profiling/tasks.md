## 1. Diagnostics-only profile contract

- [ ] 1.1 Define a dedicated diagnostics-only semantic-facts build profile for the diagnostics-only
      materialization path instead of exposing only an aggregate residual.
- [ ] 1.2 Keep full-semantic-facts observability truthful when diagnostics-only materialization
      runs, so skipped full-path leaves stay absent or zero rather than looking partially executed.

## 2. Profiling and observability wiring

- [ ] 2.1 Thread the diagnostics-only semantic-facts profile through `analysis-v2` profiled
      semantic-diagnostics APIs and representative save-follow-up profile structures.
- [ ] 2.2 Export diagnostics-only leaf attribution in runtime/backend observability surfaces and
      checked-in report payloads without overloading the old full-semantic-facts leaf names.
- [ ] 2.3 Export the traced diagnostics semantic `materialization_path` in representative
      diagnostics-save payloads and checked-in reports, not only in cumulative metrics.

## 3. Evidence and regressions

- [ ] 3.1 Add targeted regressions proving diagnostics-only profile fields appear on the
      diagnostics-only path, the traced `materialization_path` is exported, and full-only leaf
      fields stay absent or zero there.
- [ ] 3.2 Refresh representative `p55` live evidence and compare the newly attributed
      diagnostics-only residual against the `2026-04-17` `refactor-36` baseline.

## 4. Validation

- [ ] 4.1 Run targeted `analysis-v2`, `bsl-runtime`, and backend tests covering diagnostics-only
      leaf profiling and report export.
- [ ] 4.2 Run `openspec validate refactor-38-diagnostics-only-semantic-facts-leaf-profiling
      --strict --no-interactive`.
