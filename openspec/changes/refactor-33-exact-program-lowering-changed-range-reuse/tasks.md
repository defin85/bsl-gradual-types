## 1. Reuse planning for exact `program_lowering`

- [ ] 1.1 Introduce a conservative exact-path `LoweringReusePlan` derived from the previous ready
      snapshot, previous exact `ParseResult`, and current changed ranges.
- [ ] 1.2 Define explicit invalidation boundaries where reuse is forbidden and the runtime MUST
      rebuild the affected lowering region fail-closed.

## 2. Exact assembly integration

- [ ] 2.1 Wire the reuse plan into exact ready-snapshot assembly so unchanged top-level lowering
      units can be reused for local same-file edits.
- [ ] 2.2 Extend the exact path to bounded body-local reuse of unchanged sibling statement windows
      inside rebuilt routines without relaxing exact same-version guarantees.

## 3. Runtime behavior and observability

- [ ] 3.1 Preserve truthful save-critical promotion, supersession, and retarget behavior while
      reused and rebuilt lowering batches are in flight.
- [ ] 3.2 Export reuse-versus-rebuild observability for exact `program_lowering`, including
      reuse-plan outcome and bounded summaries of reused and rebuilt lowering work.
- [ ] 3.3 Add a runtime-config kill switch for the new lowering-reuse path so rollout and rollback
      do not require reverting unrelated exact-path fixes.

## 4. Evidence and regressions

- [ ] 4.1 Add targeted regressions for local same-file edits that should reuse unchanged lowering
      units, plus regressions for ambiguous edits that must fall back to rebuild.
- [ ] 4.2 Add regressions covering supersession / retarget during bounded reused-lowering batches.
- [ ] 4.3 Refresh representative `conf_big` live evidence and compare the new `program_lowering`
      residual against the current `c172fe76` baseline.

## 5. Validation

- [ ] 5.1 Run targeted backend/runtime tests covering lowering reuse, fail-closed invalidation,
      and save-follow-up exact-path behavior.
- [ ] 5.2 Run `openspec validate refactor-33-exact-program-lowering-changed-range-reuse --strict
      --no-interactive`.
