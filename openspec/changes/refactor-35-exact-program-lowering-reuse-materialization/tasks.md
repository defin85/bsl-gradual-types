## 1. Ownership-based exact reuse materialization

- [ ] 1.1 Replace clone-heavy exact reuse materialization with ownership-based consumption of
      unchanged top-level lowering units and reusable callable-body statement windows.
- [ ] 1.2 Preserve truthful reused-versus-rebuilt counters when the reuse plan is consumed, so
      observability does not depend on a second clone/rewalk pass.

## 2. Exact assembly integration

- [ ] 2.1 Thread the consumed reuse plan through parser coordination and statement conversion
      without weakening the fail-closed invalidation contract from `refactor-33`.
- [ ] 2.2 Preserve save-critical promotion, supersession, retarget, and cancellation behavior while
      exact assembly materializes reused nodes by ownership rather than deep clone.

## 3. Evidence and regressions

- [ ] 3.1 Add targeted regressions proving exact same-version semantic parity for reused top-level
      units and reused callable-body windows after ownership-based materialization.
- [ ] 3.2 Refresh representative `p53` / `p55` live evidence and compare exact
      `program_lowering_ms` against the `2026-04-17` baseline.

## 4. Validation

- [ ] 4.1 Run targeted parser/runtime/backend tests covering ownership-based reuse materialization,
      fail-closed rebuild fallback, and save-follow-up exact-path behavior.
- [ ] 4.2 Run `openspec validate refactor-35-exact-program-lowering-reuse-materialization --strict
      --no-interactive`.
