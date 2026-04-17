## 1. Callable-body partial-rebuild planning

- [x] 1.1 Define a conservative exact same-version callable-body partial-rebuild plan for one
      changed callable body, including bounded unchanged sibling-window reuse inside that body.
- [x] 1.2 Define explicit fail-closed boundaries that force whole-callable rebuild when body-local
      invalidation soundness is ambiguous.

## 2. Exact assembly integration

- [x] 2.1 Thread the callable-body partial-rebuild plan through exact ready-snapshot assembly so
      the runtime does not dispatch the whole callable body when only a bounded local region must
      rebuild.
- [x] 2.2 Preserve save-critical promotion, supersession, retarget, and cancellation behavior while
      partial callable-body rebuild is in flight.
- [x] 2.3 Export direct rebuilt callable-body observability in diagnostics-save timeline and live
      report surfaces.

## 3. Evidence and regressions

- [x] 3.1 Add targeted parser/runtime regressions proving exact semantic parity for safe
      callable-body partial rebuild.
- [x] 3.2 Add fail-closed regressions for edits whose body-local rebuild boundary is ambiguous and
      must rebuild the whole callable body.
- [x] 3.3 Refresh representative `p53` / `p55` live evidence and compare exact
      `program_lowering_ms` plus direct rebuilt callable-body metrics against the `2026-04-17`
      baseline for this follow-up.

## 4. Validation

- [x] 4.1 Run targeted parser/runtime/backend tests covering callable-body partial rebuild,
      fail-closed fallback, and exact-path orchestration invariants.
- [x] 4.2 Run `openspec validate refactor-37-exact-program-lowering-callable-body-partial-rebuild
      --strict --no-interactive`.
