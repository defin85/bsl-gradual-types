## 1. Solver contract and parity

- [x] 1.1 Define the exact correctness contract for local-function-summary inference across
      singleton non-recursive, self-recursive, and mutually recursive SCCs.
- [x] 1.2 Add targeted parity regressions proving that the optimized path preserves the same
      return-type and local-call target semantics as the pre-change exact semantic contract.

## 2. Singleton non-recursive SCC fast path

- [x] 2.1 Detect singleton SCCs without self-edges and compute their summaries in one bounded pass
      using already stabilized callee summaries.
- [x] 2.2 Keep self-recursive singleton SCCs off the fast path and on the convergence path
      fail-closed.

## 3. Recursive SCC convergence without file-wide snapshot rebuilds

- [x] 3.1 Replace per-iteration full-file local-summary snapshot rebuilds with an SCC-local overlay
      over stable out-of-SCC summaries, without cloning or rebuilding unrelated out-of-SCC entries
      per SCC or per iteration.
- [x] 3.2 Ensure fixed-point convergence, cancellation, and deterministic ordering remain coherent
      for mutually recursive SCCs under the new lookup model.

## 4. Observability and representative evidence

- [x] 4.1 Export bounded low-cardinality attribution for `local_function_summaries`, including
      prep, fixed-point, snapshot-build, body-infer, function-count, SCC-count, and convergence
      iteration count, plus explicit singleton-fast-path and recursive-SCC counters.
- [x] 4.2 Refresh representative `conf_big` live evidence and show that the residual no longer
      spends most of its time in singleton-SCC fixed-point churn.

## 5. Validation

- [x] 5.1 Run targeted `bsl-analysis-v2` and backend tests covering singleton fast path, recursive
      SCC parity, and diagnostics-save observability exports.
- [x] 5.2 Run `openspec validate refactor-34-local-function-summaries-scc-fast-path --strict
      --no-interactive`.
