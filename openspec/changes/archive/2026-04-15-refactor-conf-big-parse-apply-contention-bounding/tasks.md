## 1. Implementation
- [x] 1.1 Protect same-file current-revision `applied_version` visibility so latest `SetFile` handoff is not delayed by default behind same-file auxiliary parse/snapshot/context churn.
- [x] 1.2 Introduce same-version parse reuse/coalescing for large-module auxiliary consumers (`build_parse_snapshot_v2`, save-triggered same-version refresh, `bsl.getCurrentContext`, or semantically equivalent paths) so identical shadow text does not trigger repeated independent cold/full parse by default.
- [x] 1.3 Bound parser-side serialization impact for same-version large-file bursts using reuse, singleflight, parser partitioning, or another semantically equivalent mitigation instead of relying on a single global slow-parse queue.
- [x] 1.4 Preserve or extend low-cardinality runtime evidence so representative tests can distinguish parse-cold-start regression from writer/apply backlog without falling back to UI-side inference.

## 2. Validation
- [x] 2.1 Add deterministic regressions proving same-file current-revision waiters no longer stall on `wait_for_file_version` / `apply_lag` only because of same-file auxiliary parse churn.
- [x] 2.2 Add deterministic regressions proving large-module same-version auxiliary parse consumers reuse or coalesce parse truth instead of paying repeated identical full parse by default.
- [x] 2.3 Add representative `conf_big` mixed-load validation that separates parse mode/fallback regression from writer/apply backlog regression.
- [x] 2.4 Capture live `conf_big` evidence showing bounded current-revision apply visibility and no repeated identical same-version full parse as the default mixed-load outcome.
- [x] 2.5 Run `openspec validate refactor-conf-big-parse-apply-contention-bounding --strict --no-interactive`.
